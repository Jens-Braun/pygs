#!/bin/env python
from pygs import ufo_model, GoSamProcess, AmplitudeType, Scale
from logging import info, basicConfig
from rich.logging import RichHandler
from rich.console import Console
import numpy as np
from tqdm import tqdm

N_POINTS = 1_000_000
SCALE = 1E3
MH = 125.0
MT = 173.0

# Top Mass Range: [130, 190]

if __name__ == "__main__":

    console = Console()

    basicConfig(
        level="INFO",
        format="%(message)s",
        datefmt="[%X]",
        handlers=[
            RichHandler(show_path=False, rich_tracebacks=True),
        ],
    )

    info("Running example 'gghh'")

    info("Loading model 'models/Standard_Model_UFO'")
    m = ufo_model("models/Standard_Model_UFO")
    info("Successfully imported model 'models/Standard_Model_UFO'")

    info("Setting up process 'gghh'")
    proc = GoSamProcess(
        {"QCD": 2, "QED": 2},
        m,
        nlo_coupling = "QCD",
        contract_options = {
          "MassiveParticles": "5 6 25 24 23"
        },)

    proc.add_subprocess([21, 21], [25, 25], AmplitudeType.LoopInduced)
    with console.status("Constructing process library (this can take a few minutes)...", spinner="bouncingBall"):
        proc.setup()
    info("Successfully setup the process library")

    info(f"Sampling {N_POINTS} points from subprocess 0: 'g g -> H H'")
    rng = np.random.default_rng()
    sp_0_res = []
    #masses = []
    s = Scale.Reciprocal((2*MH)**2, (10_000)**2)
    for _ in tqdm(range(N_POINTS)):
        #mt = rng.uniform(130, 190)
        #s = Scale.Uniform((2*MH)**2, 2*(2*MT)**2)
        #proc.set_parameter("MT", mt, 0.)
        sp_0_res.append(proc.eval_random(0, s, scale = SCALE))
        #masses.append(MT)
    with open("gghh_flux.npy", "wb") as f:
        np.save(f, np.array(
            [[*np.array(l[0]).flatten().tolist(), l[1][2]] for l in sp_0_res]
        ))
    info(f"Saved {N_POINTS} point from subprocess 0 to file 'gghh_flux.npy'")
