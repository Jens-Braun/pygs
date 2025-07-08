#!/bin/env python
from pygs import ufo_model, GoSamProcess, AmplitudeType
from logging import info, basicConfig
from rich.logging import RichHandler
from rich.console import Console
import numpy as np
import timeit

N_POINTS = 1_000_000
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

    info("Running example 'uuhhuug'")

    info("Loading model 'models/Standard_Model_UFO'")
    m = ufo_model("models/Standard_Model_UFO")
    info("Successfully imported model 'models/Standard_Model_UFO'")

    info("Setting up process 'uuhhuug'")
    proc = GoSamProcess(
        {"QED": 4, "QCD": 1},
        m,
        nlo_coupling = "QCD",
        )

    proc.add_subprocess([2, -2], [25, 25, 2, -2, 21], AmplitudeType.Tree)
    with console.status("Constructing process libaray (this can take a few minutes)...", spinner="bouncingBall"):
        proc.setup()
    info("Successfully setup the process library")

    proc.sample(0, SCALE, N_POINTS)
