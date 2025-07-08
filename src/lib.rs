mod blha;
mod gosam;
mod model;
mod rambo;
mod util;

use crate::{
    gosam::{GoSamProcess, PyScale},
    model::Model,
};
use blha::AmplitudeType;
use pyo3::prelude::*;

#[pymodule]
fn pygs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    pyo3_log::init();
    m.add_class::<Model>()?;
    m.add_class::<GoSamProcess>()?;
    m.add_class::<AmplitudeType>()?;
    m.add_class::<PyScale>()?;
    Ok(())
}
