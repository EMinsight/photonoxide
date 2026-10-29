//! The rules (#544) of the thin-film lithium niobate Mach–Zehnder modulator device's cases
//! (`device::tfln_checks`). A lower fidelity against the next one up is a model, the analytic
//! fidelity's stated error, with the 2D model's own grid error beside it (device/tfln-2d-grid).
//! The published numbers take the tolerances of the same papers' cases on the cross-sections
//! (pockels/zhang-2022, pockels/wang-2018, line/zhang-2022-rf-vpi-*, line/kharel-2021-loss),
//! with the device's 2D figures gated at 1 % against grids twice as fine beside them.

use super::{Basis::*, Measure::*, Numerical, Numerical::*, Rule};

/// The 2D figures gated at 1 % against grids twice as fine.
const GATED: Numerical = Stated(1e-2, "its grids gated at 1 %, twice as fine");

/// The 2D model's own grid at Zhang et al.'s design (device/tfln-2d-grid: 4.8e-3, 2026-10-29).
const GRID: Numerical = Stated(
    4.8e-3,
    "the 2D grids against twice as fine, device/tfln-2d-grid",
);

pub(crate) fn rules() -> Vec<(&'static str, Rule)> {
    vec![
        (
            "device/tfln-analytic-vpil",
            Rule::new(
                Relative,
                Model,
                0.15,
                GRID,
                "the analytic fidelity's stated error: the field at the gap's centre across a slab mode",
            ),
        ),
        (
            "device/tfln-analytic-line",
            Rule::new(
                Relative,
                Model,
                0.1,
                GRID,
                "the analytic line's stated error: the layers' partial capacitances superposed",
            ),
        ),
        (
            "device/tfln-zhang-vpil",
            Rule::new(
                Relative,
                Reference,
                0.15 / 2.3,
                GATED,
                "as pockels/zhang-2022: r₃₃'s ±0.9 pm/V, the sidewall from 60 to 90°, the paper's two digits",
            ),
        ),
        (
            "device/tfln-wang-vpil",
            Rule::new(
                Relative,
                Reference,
                0.15 / 1.8,
                GATED,
                "as pockels/wang-2018: r₃₃'s ±0.9 pm/V, the sidewall from 60 to 90°, the paper's two digits",
            ),
        ),
        (
            "device/tfln-measured",
            Rule::new(
                Relative,
                Model,
                0.1,
                GATED,
                "the 2D fidelity's stated error against the made devices",
            ),
        ),
        (
            "device/tfln-zhang-response",
            Rule::new(
                Relative,
                Reference,
                0.2,
                GATED,
                "as line/zhang-2022-rf-vpi-short: their raw points' ±15% at 250 GHz and the line's loss",
            ),
        ),
        (
            "device/tfln-kharel-loss",
            Rule::new(
                Relative,
                Reference,
                0.12,
                GATED,
                "as line/kharel-2021-loss: the gold's ±5%, the geometry's ±3%, Holloway and Kuester's 5–10%",
            ),
        ),
        (
            "device/tfln-2d-grid",
            Rule::new(
                Relative,
                Discretisation,
                1e-2,
                Measured,
                "the static solver's and the mode's second order: halving the cells moves each figure under 1 %",
            ),
        ),
        (
            "device/tfln-jobs",
            Rule::new(
                Absolute,
                RoundOff,
                0.0,
                Exact,
                "the same computation, to the bit",
            ),
        ),
    ]
}
