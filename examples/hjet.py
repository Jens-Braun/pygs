#!/bin/env python
from pygs import ufo_model, GoSamProcess, AmplitudeType, Scale
from logging import info, basicConfig
from rich.logging import RichHandler
from rich.console import Console
import numpy as np

N_POINTS = 1_000_000
SCALE = 1E3
MH = 125.0

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

    info("Running example 'hjet'")

    info("Loading model 'models/Standard_Model_UFO'")
    m = ufo_model("models/Standard_Model_UFO")
    info("Successfully imported model 'models/Standard_Model_UFO'")

    info("Setting up process 'hjet'")
    proc = GoSamProcess(
        {"QCD": 3, "QED": 1},
        m,
        nlo_coupling = "QCD",
        contract_options = {
          "MassiveParticles": "5 6 25 24 23"
        },)

    proc.add_subprocess([21, 21], [25, 21], AmplitudeType.LoopInduced)
    with console.status("Constructing process libaray (this can take a few minutes)...", spinner="bouncingBall"):
        proc.setup()
    info("Successfully setup the process library")

    info(f"Sampling {N_POINTS} points from subprocess 0: 'g g -> H g'")
    s = Scale.Uniform(MH**2, 1E4**2)
    sp_0_res = proc.sample(0, s, N_POINTS, scale = SCALE)
    with open("hjet.npy", "wb") as f:
        np.save(f, np.array(
            [np.append(np.array(l[0]).flatten(), np.array(l[1][2])) for l in sp_0_res]
        ))
    info(f"Saved {N_POINTS} point from subprocess 0 to file 'hjet.npy'")
