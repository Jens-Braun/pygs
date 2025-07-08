#!/bin/env python
from pygs import ufo_model, GoSamProcess, AmplitudeType
from logging import info, basicConfig
from rich.logging import RichHandler
from rich.console import Console
import numpy as np

N_POINTS = 1_0
SCALE = 1E3

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

    info("Running example 'ddhh'")

    info("Loading model 'models/Standard_Model_UFO'")
    m = ufo_model("models/Standard_Model_UFO")
    info("Successfully imported model 'models/Standard_Model_UFO'")

    info("Setting up process 'ddhh'")
    quarks = ",".join(f'"{type}{id}"' for id in range(1, 5) for type in ["part", "anti"])
    proc = GoSamProcess(
        {"QED": 4},
        m,
        nlo_coupling = "EW",
        gosam_options = {
            "filter.nlo": f"lambda d: d.iprop({quarks}) == 1"
        }
        )

    proc.add_subprocess([1, -1], [25, 25], AmplitudeType.LoopInduced)
    with console.status("Constructing process libaray (this can take a few minutes)...", spinner="bouncingBall"):
        proc.setup()
    info("Successfully setup the process library")

    p1 = [500., 0., 0., 500.]
    p2 = [500., 0., 0., -500.]
    p3 = [499.9999999999999, 245.196323128194, -390.15566590687365, 148.4328787202419]
    p4 = [500.0000000000001, -245.19632312819417, 390.15566590687354, -148.4328787202419]

    res = proc.eval(0, 1000, [p1, p2, p3, p4])

    print(f"LO squared:  {res[3]}")
    print(f"NLO squared: {res[2]}")
    print(f"Single Pole: {res[1]}")
    print(f"Double Pole: {res[0]}")
