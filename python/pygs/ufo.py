import os
import sys
import importlib.util
import importlib.machinery
import cmath
from pygs import Model
from types import ModuleType

def ufo_model(path: str) -> Model:
    mod = load_ufo_files(path)
    model = Model(os.path.abspath(path))
    parameters = evaluate_parameters(mod)
    for p in mod.all_particles:
        if p.mass.name in parameters:
            mass = parameters[p.mass.name]
            if isinstance(mass, complex):
                mass = mass.real
        else:
            raise ValueError(f"Unable to determine mass of particle {p.name}")
        model.add_particle(p.pdg_code, p.name, mass, p.mass.name)
    return model

def load_ufo_files(mpath):
    mname = mpath.split("/")[-1]
    fpath = mpath + "/__init__.py"

    loader = importlib.machinery.SourceFileLoader(mname, fpath)
    spec = importlib.util.spec_from_file_location(mname, fpath, loader=loader)
    if spec is None:
        raise IOError("Unable to load UFO model")
    mod = importlib.util.module_from_spec(spec)
    sys.modules[mname] = mod
    loader.exec_module(mod)
    return mod

def evaluate_parameters(m: ModuleType) -> dict[str, float]:
    parameters = {}
    # External parameters
    for param in m.all_parameters:
        if isinstance(param.value, float | int | complex):
            parameters[param.name] = param.value
    # Internal parameters
    rerun = True
    while rerun:
        rerun = False
        for param in m.all_parameters:
            if param.name in parameters:
                continue
            if isinstance(param.value, str):
                value = eval(param.value, globals(), parameters)
                if not isinstance(value, float | int | complex):
                    continue
                parameters[param.name] = value
                rerun = True
    return parameters
