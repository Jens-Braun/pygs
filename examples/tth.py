#!/bin/env python
from pygs import ufo_model, GoSamProcess, AmplitudeType
from logging import info, basicConfig
from rich.logging import RichHandler
from rich.console import Console
import numpy as np
from numba import jit

N_POINTS = 1_000
SCALE = 1E3

MT = 173
MH = 124.96051550302843039105
S0 = (2*MT+MH)**2

def setup_parameters(proc: GoSamProcess):
    proc.set_parameter("MT", MT, 0.)
    proc.set_parameter("MH", MH, 0.)

@jit
def sp(p: np.ndarray, q: np.ndarray) -> float:
    return p[0] * q[0] - p[1] * q[1]- p[2] * q[2]- p[3] * q[3]

@jit
def beta_tt(vecs: np.ndarray) -> float:
    return np.sqrt(1. - 4*MT**2/sp(vecs[3] + vecs[4], vecs[3] + vecs[4]))

@jit
def parameterize_vecs(vecs: np.ndarray) -> np.ndarray:
    s = sp(vecs[0] + vecs[1], vecs[0] + vecs[1])
    beta_sq = 1 - S0/s
    stt = sp(vecs[3] + vecs[4], vecs[3] + vecs[4])
    fracstt = (stt - 4*MT**2)/((np.sqrt(s) - MH)**2- 4*MT**2)
    pH = np.sqrt(vecs[2][0]**2 - MH**2)
    theta_H = np.arccos(vecs[2][3]/pH)
    ptp = np.sqrt(stt/4-MT**2)
    theta_t = np.arccos(np.sqrt(stt) / ptp / pH * (vecs[3][0] - np.sqrt(pH**2 + stt)/2))
    pxp = vecs[3][3] * np.sin(theta_H) - vecs[3][1] * np.cos(theta_H)
    pyp = vecs[3][2]
    phi_t = np.arctan2(pyp, pxp)
    return np.array([1.1627906976744187 * (beta_sq-0.1), fracstt, theta_H / np.pi, theta_t / np.pi, phi_t / np.pi / 2])


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

    info("Running example 'tth'")

    info("Loading model 'models/Standard_Model_UFO'")
    m = ufo_model("models/Standard_Model_UFO")
    info("Successfully imported model 'models/Standard_Model_UFO'")

    info("Setting up process 'tth'")
    proc = GoSamProcess(
        {"QCD": 2, "QED": 1},
        m, nlo_coupling = "QCD",
        options = {
        "symmetries": "generation,family",
        "filter.lo": "lambda d: d.vertices('part6', 'anti6', 'part25') >= 1",
        "filter.nlo": "lambda d: d.vertices('part6', 'anti6', 'part25') >= 1"
        })
    proc.add_subprocess([2, -2], [25, 6, -6], AmplitudeType.Loop)
    proc.add_subprocess([21, 21], [25, 6, -6], AmplitudeType.Loop)
    proc.add_subprocess([2, -2], [25, 6, -6], AmplitudeType.ccTree)
    proc.add_subprocess([21, 21], [25, 6, -6], AmplitudeType.ccTree)
    with console.status("Constructing process libaray (this can take a few minutes)...", spinner="bouncingBall"):
        proc.setup()
    setup_parameters(proc)
    info("Successfully setup the process library")

    info(f"Sampling {N_POINTS} points from subprocess 0: 'd dbar -> h t tbar'")
    sp_0_res = proc.sample(0, SCALE, N_POINTS)
    sp_0_sub = [
        proc.eval(2, SCALE, vecs)[9] / beta_tt(np.array(vecs)) for vecs, _ in sp_0_res
    ]
    with open("subprocess_0.npy", "wb") as f:
        np.save(f, np.array(
            [np.append(np.array(l[0]).flatten(), [*np.array(l[1][2:]), sp_0_sub[i]]) for i, l in enumerate(sp_0_res)]
        ))
    info(f"Saved {N_POINTS} point from subprocess 0 to file 'subprocess_0.npy'")

    info(f"Sampling {N_POINTS} points from subprocess 1: 'g g -> h t tbar'")
    sp_1_res = proc.sample(0, SCALE, N_POINTS)
    sp_1_sub = [
        proc.eval(3, SCALE, vecs)[9] / beta_tt(np.array(vecs)) for vecs, _ in sp_1_res
    ]
    with open("subprocess_1.npy", "wb") as f:
        np.save(f, np.array(
            [np.append(np.array(l[0]).flatten(), [*np.array(l[1][2:]), sp_1_sub[i]]) for i, l in enumerate(sp_1_res)]
        ))
    info(f"Saved {N_POINTS} point from subprocess 1 to file 'subprocess_1.npy'")
