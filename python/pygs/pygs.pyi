from __future__ import annotations
from enum import Enum
from typing import Any, Optional
from math import atan

def ufo_model(path: str) -> Model:
    """Import the UFO model at the given localtion"""

class Model:
    """A basic model definition, containing only the particle ids, names and masses"""

    def __new__(cls, path: str) -> Model:
        """Create an empty model"""

    def add_particle(self, id: int, name: str, mass: float):
        """Add a particle with the given parameters to the model"""

class GoSamProcess:
    """A process backed by GoSam"""

    def __new__(cls,
        coupling_orders: dict[str, int],
        model: Model,
        nlo_coupling: Optional[str] = None,
        contract_options: Optional[dict[str, str]] = None,
        gosam_options: Optional[dict[str, str]] = None
    ) -> GoSamProcess:
        "Create a new GoSam process"

    def add_subprocess(self, incoming: list[int], outgoing: list[int], amplitude_type: AmplitudeType):
        """Add a new subprocess"""

    def setup(self):
        """
        Generate, compile and load the process library. The library will be reused for subsequent runs if the
        configuration is the same.
        """

    def set_parameter(self, parameter: str, real: float, imag: float):
        """Set parameter `parameter` to value `real + i float`"""

    def eval(self, id: int, scale: float, vecs: list[list[float]]) -> list[float]:
        """
        Evaluate subprocess `id` with energy scale `scale` at phase space point `vecs`

        Parameters:
            id: identifier of the subprocess
            scale: energy scale to evaluate at
            vecs: phase space point as list of four-vectors, where each four-vector is a list of exactly four floats

        Returns:
            list of floats with length depending on the amplitude type, see BLHA2 standard (1308.3462) for details
        """

    def sample(self, id: int, s: Scale, n_points: int, scale: Optional[float] = None) -> list[tuple[list[list[float]], list[float]]]:
        """
        Evaluate subprocess `id` with energy scale `scale` at `n_points` random phase-space points (constructed by a
        RAMBO generator).

        Parameters:
            id: identifier of the subprocess
            s: center of mass energy squared of the sampled points
            n_points: number of points to sample
            scale: Renormalization scale to evaluate the amplitude at (default: center of mass energy)

        Returns:
            list of sampled points, where each entry contains the phase space point and the result. See also: [eval]
        """

class AmplitudeType(Enum):
    """
    The BLHA2 amplitude type. Possible values:

    - Tree
    - scTree
    - scTree2
    - ccTree
    - Loop
    - LoopInduced
    """
    Tree: ...
    scTree: ...
    scTree2: ...
    ccTree: ...
    Loop: ...
    LoopInduced: ...

class Scale(Enum):
    """
    Type of energy scale. Possible values:

    - Fixed: fixed energy scale
    - Uniform: uniformly sampled energy scale in some range
    """
    Fixed: ...
    Uniform:...
