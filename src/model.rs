use std::{collections::HashMap, path::PathBuf};

use pyo3::{pyclass, pymethods};

#[pyclass]
#[derive(Clone, Debug)]
pub(crate) struct Model {
    pub(crate) path: PathBuf,
    pub(crate) particles: HashMap<i64, Particle>,
}

impl Model {
    pub(crate) fn get_mass(&self, id: i64) -> f64 {
        return self.particles.get(&id).unwrap().mass;
    }

    pub(crate) fn update_mass(&mut self, ident: &str, value: f64) {
        for p in self.particles.values_mut() {
            if p.mass_ident == ident {
                p.mass = value
            }
        }
    }
}

#[pymethods]
impl Model {
    #[new]
    fn new(path: PathBuf) -> Model {
        Model {
            path,
            particles: HashMap::new(),
        }
    }

    fn add_particle(&mut self, pdg_id: i64, name: String, mass: f64, mass_ident: String) {
        self.particles.insert(
            pdg_id,
            Particle {
                pdg_id,
                name,
                mass,
                mass_ident,
            },
        );
    }

    fn __str__(&self) -> String {
        return format!("{self:#?}");
    }

    fn __repr__(&self) -> String {
        return format!("{self:?}");
    }
}

#[derive(Clone, Debug)]
struct Particle {
    pdg_id: i64,
    name: String,
    mass: f64,
    mass_ident: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn model_test() {
        let mut m = Model::new(PathBuf::from("/tmp"));
        m.add_particle(6, "t".into(), 172., "MT".into());
        m.add_particle(-6, "t~".into(), 172., "MT".into());
        m.add_particle(1, "d".into(), 0., "MD".into());

        m.update_mass("MT", 173.);

        assert_eq!(m.particles[&-6].mass, 173.);
    }
}