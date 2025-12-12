use crate::{
    blha::{AmplitudeType, OneLoopProvider, Order, Subprocess, error::BLHAError},
    model::Model,
    rambo::{Scale, rambo},
};
use indexmap::IndexMap;
use pyo3::types::IntoPyDict;
use std::hash::{DefaultHasher, Hash};
use std::io::Write;
use std::{hash::Hasher, path::PathBuf};
use thiserror::Error;

use pyo3::{
    exceptions::{PyIOError, PySyntaxError},
    prelude::*,
};

#[derive(Error, Debug)]
enum GoSamError {
    #[error("Error while generating the process library: {0}")]
    GenError(String),
    #[error("Process has to be initialized before {0} is available")]
    UnintializedError(String),
    #[error("IOError: {0}")]
    IOError(#[from] std::io::Error),
    #[error(transparent)]
    BLHAError(#[from] BLHAError),
}

impl From<GoSamError> for PyErr {
    fn from(err: GoSamError) -> PyErr {
        match err {
            GoSamError::GenError(_) => PySyntaxError::new_err(err.to_string()),
            GoSamError::UnintializedError(_) => PySyntaxError::new_err(err.to_string()),
            GoSamError::BLHAError(_) => PySyntaxError::new_err(err.to_string()),
            GoSamError::IOError(_) => PyIOError::new_err(err.to_string()),
        }
    }
}

impl From<BLHAError> for PyErr {
    fn from(err: BLHAError) -> PyErr {
        match err {
            BLHAError::IOError(_, _) => PyIOError::new_err(err.to_string()),
            BLHAError::OLPError(_, _) => PySyntaxError::new_err(err.to_string()),
            BLHAError::ContractError(_) => PySyntaxError::new_err(err.to_string()),
            BLHAError::ParseError(_, _) => PySyntaxError::new_err(err.to_string()),
            BLHAError::LibraryError(_) => PyIOError::new_err(err.to_string()),
        }
    }
}

#[pyclass]
#[pyo3(name = "Scale")]
#[derive(Clone)]
pub(crate) enum PyScale {
    Fixed(f64),
    Uniform { min: f64, max: f64 },
    Reciprocal { min: f64, max: f64 },
}

impl From<&PyScale> for Scale<f64> {
    fn from(value: &PyScale) -> Self {
        match value {
            PyScale::Fixed(s) => Scale::Fixed(*s),
            PyScale::Uniform { min, max } => Scale::Uniform {
                min: *min,
                max: *max,
            },
            PyScale::Reciprocal { min, max } => Scale::Reciprocal {
                min: *min,
                max: *max,
            },
        }
    }
}

#[pyclass]
pub(crate) struct GoSamProcess {
    coupling_orders: IndexMap<String, usize>,
    nlo_coupling: Option<String>,
    contract_options: Option<IndexMap<String, String>>,
    gosam_options: Option<IndexMap<String, String>>,
    subprocesses: Vec<Subprocess>,
    model: Model,
    olp: Option<OneLoopProvider>,
}

impl Hash for GoSamProcess {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.coupling_orders.iter().collect::<Vec<_>>().hash(state);
        if let Some(ref opts) = self.contract_options {
            opts.iter().collect::<Vec<_>>().hash(state);
        }
        if let Some(ref opts) = self.gosam_options {
            opts.iter().collect::<Vec<_>>().hash(state);
        }
        self.nlo_coupling.hash(state);
        self.subprocesses.hash(state);
    }
}

impl GoSamProcess {
    fn write_order(&self) -> Result<(), GoSamError> {
        let options = if let Some(ref contract_options) = self.contract_options {
            let mut tmp = contract_options.clone();
            tmp.insert(
                "CorrectionType".to_owned(),
                if let Some(ref nlo) = self.nlo_coupling {
                    nlo.clone()
                } else {
                    "QCD".to_owned()
                },
            );
            tmp
        } else {
            IndexMap::from([(
                "CorrectionType".to_owned(),
                if let Some(ref nlo) = self.nlo_coupling {
                    nlo.clone()
                } else {
                    "QCD".to_owned()
                },
            )])
        };
        let order = Order {
            coupling_orders: self.coupling_orders.clone(),
            model: &self.model,
            nlo_coupling: self.nlo_coupling.clone(),
            subprocesses: &self.subprocesses,
            options,
        };
        crate::blha::order_writer::write_order_file(
            &order,
            &std::env::current_dir()?.join("gosam.olp"),
        )?;
        if let Some(ref options) = self.gosam_options {
            let mut config = std::fs::File::create(&std::env::current_dir()?.join("gosam.in"))?;
            for (option, value) in options.iter() {
                writeln!(config, "{}={}", option, value)?;
            }
        }
        Ok(())
    }

    fn run_gosam(&self) -> Result<(), GoSamError> {
        let res = std::process::Command::new("gosam.py")
            .args(["--olp", "gosam.olp", "-I", "-f", "-z"])
            .output()?;
        if !(res.status.code() == Some(0)) {
            return Err(GoSamError::GenError(String::from_utf8(res.stdout).unwrap()));
        }
        Ok(())
    }

    fn compile_process_libaray(&self) -> Result<(), GoSamError> {
        std::process::Command::new("meson")
            .args([
                "setup",
                "build",
                "--prefix",
                std::env::current_dir().unwrap().to_str().unwrap(),
            ])
            .output()?;
        std::process::Command::new("meson")
            .args(["compile", "-C", "build"])
            .output()?;
        Ok(())
    }

    fn setup_process(&mut self) -> Result<(), GoSamError> {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        let hash = hasher.finish();
        if !std::fs::exists("gosam_process")? {
            std::fs::create_dir("gosam_process")?;
        }
        let working_path = std::env::current_dir()?;
        let process_path = PathBuf::from("gosam_process").join(hash.to_string());
        if !std::fs::exists(&process_path)? {
            std::fs::create_dir(&process_path)?;
        }
        if std::fs::exists(&process_path.join("build/libgolem_olp.so"))? {
            self.olp = Some(OneLoopProvider::new(
                &process_path.join("gosam.olc"),
                &process_path.join("build/libgolem_olp.so"),
            )?);
            return Ok(());
        }
        std::env::set_current_dir(&process_path)?;
        self.write_order()?;
        self.run_gosam()?;
        self.compile_process_libaray()?;
        self.olp = Some(OneLoopProvider::new(
            &std::env::current_dir()?.join("gosam.olc"),
            &std::env::current_dir()?.join("build/libgolem_olp.so"),
        )?);
        std::env::set_current_dir(working_path)?;
        Ok(())
    }
}

#[pymethods]
impl GoSamProcess {
    #[new]
    #[pyo3(signature = (coupling_orders, model, nlo_coupling = None, contract_options = None, gosam_options = None))]
    fn new(
        coupling_orders: IndexMap<String, usize>,
        model: Model,
        nlo_coupling: Option<String>,
        contract_options: Option<IndexMap<String, Bound<'_, PyAny>>>,
        gosam_options: Option<IndexMap<String, Bound<'_, PyAny>>>,
    ) -> PyResult<Self> {
        let contract_opts;
        if let Some(options) = contract_options {
            let mut map = IndexMap::with_capacity(options.len());
            for (key, value) in options.into_iter() {
                map.insert(key, value.str()?.extract::<String>()?);
            }
            contract_opts = Some(map);
        } else {
            contract_opts = None;
        }
        let gs_opts;
        if let Some(options) = gosam_options {
            let mut map = IndexMap::with_capacity(options.len());
            for (key, value) in options.into_iter() {
                map.insert(key, value.str()?.extract::<String>()?);
            }
            gs_opts = Some(map);
        } else {
            gs_opts = None;
        }
        Ok(GoSamProcess {
            coupling_orders,
            nlo_coupling,
            contract_options: contract_opts,
            gosam_options: gs_opts,
            subprocesses: vec![],
            model,
            olp: None,
        })
    }

    fn add_subprocess(
        &mut self,
        incoming: Vec<i64>,
        outgoing: Vec<i64>,
        amplitude_type: AmplitudeType,
    ) {
        let id = self.subprocesses.len() as i64;
        self.subprocesses.push(Subprocess {
            id,
            incoming_pdg: incoming,
            outgoing_pdg: outgoing,
            amplitude_type,
        });
    }

    fn setup(&mut self, py: Python<'_>) -> PyResult<()> {
        py.allow_threads(|| -> Result<(), GoSamError> { self.setup_process() })?;
        Ok(())
    }

    fn set_parameter(&mut self, parameter: String, real: f64, imag: f64) -> PyResult<()> {
        if let Some(ref olp) = self.olp {
            self.model.update_mass(&parameter, real);
            return Ok(olp.set_parameter(&parameter, real, imag)?);
        } else {
            return Err(GoSamError::UnintializedError("set_parameter".to_owned()))?;
        }
    }

    fn print_parameters(&self, filename: String) -> PyResult<()> {
        if let Some(ref olp) = self.olp {
            return Ok(olp.print_parameters(&filename));
        } else {
            return Err(GoSamError::UnintializedError("print_parameters".to_owned()))?;
        }
    }

    fn eval(
        &self,
        py: Python<'_>,
        id: usize,
        scale: f64,
        vecs: Vec<[f64; 4]>,
    ) -> PyResult<Vec<f64>> {
        if let Some(ref olp) = self.olp {
            return Ok(py.allow_threads(|| -> Result<_, _> { olp.eval(id, &vecs, scale) })?);
        } else {
            return Err(GoSamError::UnintializedError("eval".to_owned()))?;
        }
    }

    #[pyo3(signature = (id, s, scale = None))]
    fn eval_random(
        &self,
        py: Python<'_>,
        id: usize,
        s: PyScale,
        scale: Option<f64>,
    ) -> PyResult<(Vec<[f64; 4]>, Vec<f64>)> {
        let lib;
        if let Some(ref olp) = self.olp {
            lib = olp;
        } else {
            return Err(GoSamError::UnintializedError("eval".to_owned()))?;
        }
        let masses = self.subprocesses[id]
            .incoming_pdg
            .iter()
            .chain(self.subprocesses[id].outgoing_pdg.iter())
            .map(|i| self.model.get_mass(*i))
            .collect::<Vec<_>>();
        let n_in = self.subprocesses[id].incoming_pdg.len();
        let mut rng = fastrand::Rng::new();

        let result = py.allow_threads(|| -> PyResult<_> {
            let (mut renorm_scale, vecs) = rambo((&s).into(), &masses, n_in, &mut rng);
            if let Some(scale) = scale {
                renorm_scale = scale;
            }
            let vals = lib.eval(id, &vecs, renorm_scale)?;
            return Ok((vecs, vals));
        });
        return result;
    }

    #[pyo3(signature = (id, s, n_points, scale = None))]
    fn sample(
        &self,
        py: Python<'_>,
        id: usize,
        s: PyScale,
        n_points: usize,
        scale: Option<f64>,
    ) -> PyResult<Vec<(Vec<[f64; 4]>, Vec<f64>)>> {
        let lib;
        if let Some(ref olp) = self.olp {
            lib = olp;
        } else {
            return Err(GoSamError::UnintializedError("eval".to_owned()))?;
        }
        let masses = self.subprocesses[id]
            .incoming_pdg
            .iter()
            .chain(self.subprocesses[id].outgoing_pdg.iter())
            .map(|i| self.model.get_mass(*i))
            .collect::<Vec<_>>();
        let n_in = self.subprocesses[id].incoming_pdg.len();
        let mut rng = fastrand::Rng::new();

        let tqdm = match py.import("tqdm.auto") {
            Ok(m) => Some(
                m.getattr("tqdm")?
                    .call((), Some(&[("total", n_points)].into_py_dict(py)?))?
                    .unbind(),
            ),
            Err(_) => None,
        };
        let mut update = None;
        if let Some(ref tqdm) = tqdm {
            update =
                Some(|inc: usize| Python::with_gil(|py| tqdm.call_method1(py, "update", (inc,))))
        }
        let n_update = if n_points >= 1000 { n_points / 1000 } else { 1 };
        let result = py.allow_threads(|| -> PyResult<_> {
            let mut result = Vec::with_capacity(n_points);
            for i in 0..n_points {
                let (mut renorm_scale, vecs) = rambo((&s).into(), &masses, n_in, &mut rng);
                if let Some(scale) = scale {
                    renorm_scale = scale;
                }
                let vals = lib.eval(id, &vecs, renorm_scale)?;
                result.push((vecs, vals));
                if i % n_update == 0 {
                    if let Some(f) = update {
                        f(n_update)?;
                    }
                }
            }
            return Ok(result);
        });
        if let Some(ref tqdm) = tqdm {
            tqdm.call_method0(py, "close")?;
        }
        return result;
    }
}
