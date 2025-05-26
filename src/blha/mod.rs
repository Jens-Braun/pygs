#![allow(non_camel_case_types)]

use std::{
    ffi::{CString, c_char, c_double},
    fmt::Display,
    path::Path,
};

use indexmap::IndexMap;
use libloading::{Library, Symbol};
use pyo3::prelude::*;
use self_cell::self_cell;

use error::BLHAError;

use crate::{model::Model, util::scalar};

pub(crate) mod error;
pub(crate) mod order_writer;
mod parser;

#[derive(Debug, Clone, Copy, PartialEq, Hash)]
#[pyclass]
pub(crate) enum AmplitudeType {
    Tree,
    scTree,
    scTree2,
    ccTree,
    Loop,
    LoopInduced,
}

impl Display for AmplitudeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tree => write!(f, "Tree"),
            Self::scTree => write!(f, "scTree"),
            Self::scTree2 => write!(f, "scTree2"),
            Self::ccTree => write!(f, "ccTree"),
            Self::Loop => write!(f, "Loop"),
            Self::LoopInduced => write!(f, "LoopInduced"),
        }
    }
}

#[derive(Debug, PartialEq, Hash)]
pub(crate) struct Subprocess {
    pub(crate) id: i64,
    pub(crate) amplitude_type: AmplitudeType,
    pub(crate) incoming_pdg: Vec<i64>,
    pub(crate) outgoing_pdg: Vec<i64>,
}

impl Subprocess {
    fn n_legs(&self) -> usize {
        self.incoming_pdg.len() + self.outgoing_pdg.len()
    }
}

#[derive(Debug, PartialEq)]
struct Contract {
    options: IndexMap<String, String>,
    subprocesses: Vec<Subprocess>,
}

pub(crate) struct Order<'a> {
    pub(crate) model: &'a Model,
    pub(crate) coupling_orders: IndexMap<String, usize>,
    pub(crate) nlo_coupling: Option<String>,
    pub(crate) options: IndexMap<String, String>,
    pub(crate) subprocesses: &'a Vec<Subprocess>,
}

struct BLHAInterface<'a> {
    start: Symbol<'a, unsafe extern "C" fn(*const c_char, *mut i32)>,
    info: Symbol<'a, unsafe extern "C" fn(*const c_char, *const c_char, *mut c_char)>,
    set_parameter:
        Symbol<'a, unsafe extern "C" fn(*const c_char, *const c_double, *const c_double, *mut i32)>,
    print_parameters: Symbol<'a, unsafe extern "C" fn(*const c_char)>,
    eval: Symbol<
        'a,
        unsafe extern "C" fn(
            *const i32,
            *const c_double,
            *const c_double,
            *mut c_double,
            *mut c_double,
        ),
    >,
}

self_cell!(
    struct OLPLibrary {
        owner: Library,
        #[covariant]
        dependent: BLHAInterface,
    }
);

pub(crate) struct OneLoopProvider {
    contract: Contract,
    lib: OLPLibrary,
}

impl OneLoopProvider {
    pub(crate) fn new(contract_path: &Path, library_path: &Path) -> Result<Self, BLHAError> {
        let contract = parser::parse_contract(contract_path)?;
        let library;
        unsafe {
            library = Library::new(library_path)?;
        }

        let olp = Self {
            contract,
            lib: OLPLibrary::try_new(library, |lib| -> Result<BLHAInterface<'_>, BLHAError> {
                Ok(BLHAInterface {
                    start: unsafe { lib.get(b"OLP_Start")? },
                    info: unsafe { lib.get(b"OLP_Info")? },
                    set_parameter: unsafe { lib.get(b"OLP_SetParameter")? },
                    print_parameters: unsafe { lib.get(b"OLP_PrintParameter")? },
                    eval: unsafe { lib.get(b"OLP_EvalSubProcess2")? },
                })
            })?,
        };
        olp.start(contract_path)?;
        return Ok(olp);
    }

    pub(crate) fn start(&self, contract: &Path) -> Result<(), BLHAError> {
        let contract_string = CString::new(contract.to_str().unwrap()).unwrap();
        let mut ierr: i32 = 1;
        unsafe {
            (self.lib.borrow_dependent().start)(contract_string.as_ptr(), &mut ierr as *mut i32);
        }
        if ierr != 1 {
            return Err(BLHAError::OLPError("OLP_Start".into(), ierr));
        }
        return Ok(());
    }

    pub(crate) fn set_parameter(
        &self,
        parameter: &str,
        real: f64,
        imag: f64,
    ) -> Result<(), BLHAError> {
        let mut ierr: i32 = 1;
        unsafe {
            (self.lib.borrow_dependent().set_parameter)(
                CString::new(parameter).unwrap().as_ptr(),
                &real as *const f64,
                &imag as *const f64,
                &mut ierr as *mut i32,
            )
        }
        if ierr != 1 {
            return Err(BLHAError::OLPError("OLP_Start".into(), ierr));
        }
        return Ok(());
    }

    pub(crate) fn print_parameters(&self, filename: &str) {
        unsafe {
            (self.lib.borrow_dependent().print_parameters)(
                CString::new(filename).unwrap().as_ptr(),
            );
        }
    }

    pub(crate) fn eval(
        &self,
        id: usize,
        momenta: &[[f64; 4]],
        scale: f64,
    ) -> Result<Vec<f64>, BLHAError> {
        let n_results = match self.contract.subprocesses[id].amplitude_type {
            AmplitudeType::Tree | AmplitudeType::LoopInduced => 4,
            AmplitudeType::Loop => 4,
            AmplitudeType::ccTree => {
                let n = self.contract.subprocesses[id].n_legs();
                n * (n - 1) / 2
            }
            AmplitudeType::scTree | AmplitudeType::scTree2 => {
                let n = self.contract.subprocesses[id].n_legs();
                2 * n * n
            }
        };
        let mut res = vec![0.; n_results];
        let mut momenta_flat = vec![0.; 5 * momenta.len()];
        for (i, momentum) in momenta.iter().enumerate() {
            for j in 0..=4 {
                momenta_flat[5 * i + j] = if j == 4 {
                    scalar(momentum, momentum)
                } else {
                    momentum[j]
                }
            }
        }
        let mut precision = 0.;
        unsafe {
            (self.lib.borrow_dependent().eval)(
                &(id as i32) as *const i32,
                momenta_flat.as_ptr(),
                &scale as *const f64,
                res.as_mut_ptr(),
                &mut precision as *mut f64,
            )
        }
        return Ok(res);
    }
}
