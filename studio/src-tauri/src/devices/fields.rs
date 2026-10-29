//! The fields a device's page and the run view draw on a cross-section, beyond its
//! permittivity:
//!
//! - [`device_temperature`]: a thermo-optic phase shifter's temperature rise at the design's
//!   power (at the power for a π shift when the design's is zero), by its 2D model
//!   (`photonoxide::thermal::PhaseShifter`, the static solver's heat on the device's own
//!   cross-section), with its isotherms, the media's edges and where the mode takes its phase
//!   from (its thermo-optic weights);
//! - [`raster_contours`]: the lines at given levels of any picture, for the run view's
//!   isotherms and the outline of a static job's mode over its electrodes' field.
//!
//! The lines are found on the picture's pixels (marching squares between the pixels' centres),
//! so they are as fine as the picture, not the solver's grid.

use std::collections::HashMap;

use photonoxide::device::{Device, ThermoShifter};
use photonoxide::raster::Raster;
use photonoxide::thermal::{Field, PhaseShifter, Section};
use serde::Serialize;

use super::{SpecIn, text};

/// A line on a picture: its points (µm, the picture's coordinates).
pub type Line = Vec<[f64; 2]>;

/// The lines where a picture takes one value.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Isoline {
    pub level: f64,
    pub lines: Vec<Line>,
}

/// Where along the edge from value `a` to value `b` the value `level` is, from 0 to 1.
fn crossing(a: f64, b: f64, level: f64) -> f64 {
    if a == b {
        0.5
    } else {
        ((level - a) / (b - a)).clamp(0.0, 1.0)
    }
}

/// The lines where `raster` takes `level`, between its pixels' centres: marching squares, a
/// saddle resolved by its cell's mean, the segments joined into lines (closed ones end where
/// they start). A cell with a value that isn't finite has no line.
pub fn contours(raster: &Raster, level: f64) -> Vec<Line> {
    let (nx, ny) = (raster.nx, raster.ny);
    if nx < 2 || ny < 2 || !level.is_finite() {
        return Vec::new();
    }
    let (dx, dy) = (
        (raster.x1 - raster.x0) / nx as f64,
        (raster.y1 - raster.y0) / ny as f64,
    );
    let v = |i: usize, j: usize| f64::from(raster.values[j * nx + i]);
    let centre = |i: usize, j: usize| {
        [
            raster.x0 + (i as f64 + 0.5) * dx,
            raster.y0 + (j as f64 + 0.5) * dy,
        ]
    };
    // an edge between two neighbouring centres: (0, i, j) from (i, j) to (i + 1, j), (1, i, j)
    // from (i, j) to (i, j + 1)
    type Key = (u8, usize, usize);
    let point = |k: Key| -> [f64; 2] {
        let (i, j) = (k.1, k.2);
        let (a, b) = if k.0 == 0 {
            ((i, j), (i + 1, j))
        } else {
            ((i, j), (i, j + 1))
        };
        let t = crossing(v(a.0, a.1), v(b.0, b.1), level);
        let (p, q) = (centre(a.0, a.1), centre(b.0, b.1));
        [p[0] + t * (q[0] - p[0]), p[1] + t * (q[1] - p[1])]
    };
    let mut segments: Vec<[Key; 2]> = Vec::new();
    for j in 0..ny - 1 {
        for i in 0..nx - 1 {
            let corners = [v(i, j), v(i + 1, j), v(i + 1, j + 1), v(i, j + 1)];
            if corners.iter().any(|c| !c.is_finite()) {
                continue;
            }
            let case = corners
                .iter()
                .enumerate()
                .fold(0u8, |c, (k, &x)| c | (u8::from(x > level) << k));
            // the cell's edges: bottom, right, top, left
            let edge = [(0, i, j), (1, i + 1, j), (0, i, j + 1), (1, i, j)];
            let inside = corners.iter().sum::<f64>() / 4.0 > level;
            let pairs: &[(usize, usize)] = match case {
                1 | 14 => &[(3, 0)],
                2 | 13 => &[(0, 1)],
                3 | 12 => &[(3, 1)],
                4 | 11 => &[(1, 2)],
                6 | 9 => &[(0, 2)],
                7 | 8 => &[(3, 2)],
                // the saddles: the corners the centre isn't with are cut off
                5 if inside => &[(0, 1), (2, 3)],
                5 => &[(3, 0), (1, 2)],
                10 if inside => &[(3, 0), (1, 2)],
                10 => &[(0, 1), (2, 3)],
                _ => &[],
            };
            for &(a, b) in pairs {
                segments.push([edge[a], edge[b]]);
            }
        }
    }
    // each edge has at most two segments: chain them
    let mut at: HashMap<Key, Vec<usize>> = HashMap::new();
    for (s, seg) in segments.iter().enumerate() {
        for k in seg {
            at.entry(*k).or_default().push(s);
        }
    }
    let mut used = vec![false; segments.len()];
    let mut lines = Vec::new();
    for s in 0..segments.len() {
        if used[s] {
            continue;
        }
        used[s] = true;
        let mut chain = std::collections::VecDeque::from([segments[s][0], segments[s][1]]);
        for forwards in [true, false] {
            loop {
                let end = if forwards {
                    *chain.back().expect("a chain has two ends")
                } else {
                    *chain.front().expect("a chain has two ends")
                };
                let Some(&t) = at[&end].iter().find(|&&t| !used[t]) else {
                    break;
                };
                used[t] = true;
                let next = if segments[t][0] == end {
                    segments[t][1]
                } else {
                    segments[t][0]
                };
                if forwards {
                    chain.push_back(next);
                } else {
                    chain.push_front(next);
                }
            }
        }
        lines.push(chain.into_iter().map(point).collect());
    }
    lines
}

/// The lines where a picture takes each of `levels`.
#[tauri::command(async)]
pub fn raster_contours(raster: Raster, levels: Vec<f64>) -> Vec<Isoline> {
    levels
        .into_iter()
        .map(|level| Isoline {
            level,
            lines: contours(&raster, level),
        })
        .collect()
}

/// Round levels from above 0 to below `peak`, about `count` of them: 1, 2 or 5 times a power
/// of ten apart.
pub fn levels(peak: f64, count: usize) -> Vec<f64> {
    if !(peak.is_finite() && peak > 0.0) || count == 0 {
        return Vec::new();
    }
    let rough = peak / count as f64;
    let ten = 10f64.powf(rough.log10().floor());
    let step = [1.0, 2.0, 5.0, 10.0]
        .into_iter()
        .map(|m| m * ten)
        .find(|s| *s >= rough)
        .unwrap_or(10.0 * ten);
    (1..)
        .map(|k| k as f64 * step)
        .take_while(|l| *l < peak * (1.0 - 1e-9))
        .collect()
}

/// Where the media change between a picture's pixels over `x` × `y` (µm), `nx` × `ny` of them:
/// each change along a row or a column of pixel edges as one straight line, for the media's
/// edges drawn over a field.
pub fn edges(section: &Section, x: [f64; 2], y: [f64; 2], nx: usize, ny: usize) -> Vec<Line> {
    let (dx, dy) = ((x[1] - x[0]) / nx as f64, (y[1] - y[0]) / ny as f64);
    let medium: Vec<&str> = (0..ny)
        .flat_map(|j| {
            (0..nx).map(move |i| {
                section.medium_at(x[0] + (i as f64 + 0.5) * dx, y[0] + (j as f64 + 0.5) * dy)
            })
        })
        .collect();
    let m = |i: usize, j: usize| medium[j * nx + i];
    let mut lines = Vec::new();
    // between rows j − 1 and j: runs along x
    for j in 1..ny {
        let mut start = None;
        for i in 0..=nx {
            let differs = i < nx && m(i, j - 1) != m(i, j);
            match (differs, start) {
                (true, None) => start = Some(i),
                (false, Some(s)) => {
                    let yy = y[0] + j as f64 * dy;
                    lines.push(vec![[x[0] + s as f64 * dx, yy], [x[0] + i as f64 * dx, yy]]);
                    start = None;
                }
                _ => {}
            }
        }
    }
    // between columns i − 1 and i: runs along y
    for i in 1..nx {
        let mut start = None;
        for j in 0..=ny {
            let differs = j < ny && m(i - 1, j) != m(i, j);
            match (differs, start) {
                (true, None) => start = Some(j),
                (false, Some(s)) => {
                    let xx = x[0] + i as f64 * dx;
                    lines.push(vec![[xx, y[0] + s as f64 * dy], [xx, y[0] + j as f64 * dy]]);
                    start = None;
                }
                _ => {}
            }
        }
    }
    lines
}

/// A thermo-optic phase shifter's temperature, as its page's Temperature view shows it: at the
/// design's power, or at the power for a π shift when the design's is zero.
#[derive(Debug, Serialize)]
pub struct Temperature {
    /// The rise above the ambient, K: x across the guide (0 at its centre), y up (0 at the
    /// buried oxide's top), µm.
    pub raster: Raster,
    /// Where the media change on the picture.
    pub outline: Vec<Line>,
    /// The isotherms, K: round levels up to the peak.
    pub isotherms: Vec<Isoline>,
    /// Where the guide's mode takes its phase from the heat: its thermo-optic weights at half
    /// and 1/e² of their largest (the levels as those shares, 0.5 and 0.135).
    pub mode: Vec<Isoline>,
    /// The power the picture is at, mW: the design's, or P_π when the design's is zero
    /// (`at_pi`).
    pub power_mw: f64,
    pub at_pi: bool,
    /// The 2D model's power for a π shift, mW (λ / 2s, s from the same rise and weights).
    pub p_pi_mw: f64,
    /// The drive per unit length, W/m: the power over the design's length.
    pub watts_per_metre: f64,
    /// The largest rise, K, and where it is (µm).
    pub peak: f64,
    pub peak_at: [f64; 2],
    /// The rise the mode feels, K: its weights' mean of the rise, what its index follows.
    pub guide: f64,
    /// The heat problem's grid and the mode's, in words.
    pub grid: String,
}

/// The most pixels across or up the temperature's picture.
const PIXELS: f64 = 480.0;
/// How far the picture reaches past what is drawn (the guide, the heaters), µm.
const MARGIN: f64 = 1.5;

/// The thermo-optic shifter `spec` designs, at its stack and core.
pub(crate) fn thermo_shifter(spec: &photonoxide::device::Spec) -> Result<ThermoShifter, String> {
    let (stack, core) = spec.stack.build().map_err(text)?;
    match spec.kind.as_str() {
        "thermo-optic phase shifter" => ThermoShifter::on(stack, &core, false),
        "doped thermo-optic phase shifter" => ThermoShifter::on(stack, &core, true),
        "in-guide thermo-optic phase shifter" => ThermoShifter::in_guide_on(stack, &core),
        other => {
            return Err(format!(
                "a {other} has no heat model: the temperature is a thermo-optic phase shifter's"
            ));
        }
    }
    .map_err(text)
}

/// What the cross-section draws besides its layers: the cells whose medium isn't the one at the
/// window's left edge at their height, as (x, y) bounds, µm.
fn drawn(section: &Section, rise: &Field) -> Option<([f64; 2], [f64; 2])> {
    let (gx, gy) = (rise.grid().x(), rise.grid().y());
    let left = gx.centre(0);
    let (mut x, mut y) = (
        [f64::INFINITY, f64::NEG_INFINITY],
        [f64::INFINITY, f64::NEG_INFINITY],
    );
    for j in 0..gy.cells() {
        let yc = gy.centre(j);
        let layer = section.medium_at(left, yc);
        for i in 0..gx.cells() {
            let xc = gx.centre(i);
            if section.medium_at(xc, yc) != layer {
                let (hx, hy) = (0.5 * gx.width(i), 0.5 * gy.width(j));
                x = [x[0].min(xc - hx), x[1].max(xc + hx)];
                y = [y[0].min(yc - hy), y[1].max(yc + hy)];
            }
        }
    }
    x[0].is_finite().then_some((x, y))
}

/// The guide's weights as a picture on their own uniform grid, a cell of zeros around them so
/// their lines close, and its step (µm).
fn weights_picture(weights: &[([f64; 2], f64)]) -> Option<(Raster, f64)> {
    let mut xs: Vec<f64> = weights.iter().map(|w| w.0[0]).collect();
    let mut ys: Vec<f64> = weights.iter().map(|w| w.0[1]).collect();
    let step = |v: &mut Vec<f64>| {
        v.sort_by(f64::total_cmp);
        v.windows(2)
            .map(|p| p[1] - p[0])
            .filter(|d| *d > 1e-9)
            .fold(f64::INFINITY, f64::min)
    };
    let h = step(&mut xs).min(step(&mut ys));
    if !h.is_finite() {
        return None;
    }
    let (x0, y0) = (xs[0] - h, ys[0] - h);
    let nx = ((xs[xs.len() - 1] - xs[0]) / h).round() as usize + 3;
    let ny = ((ys[ys.len() - 1] - ys[0]) / h).round() as usize + 3;
    let mut values = vec![0.0f32; nx * ny];
    for &([x, y], w) in weights {
        let i = ((x - x0) / h).round() as usize;
        let j = ((y - y0) / h).round() as usize;
        values[j * nx + i] = w.abs() as f32;
    }
    Some((
        Raster {
            nx,
            ny,
            x0: x0 - 0.5 * h,
            x1: x0 - 0.5 * h + nx as f64 * h,
            y0: y0 - 0.5 * h,
            y1: y0 - 0.5 * h + ny as f64 * h,
            values,
        },
        h,
    ))
}

/// `spec`'s temperature by its 2D model at its power (P_π when its power is zero): see
/// [`Temperature`].
///
/// # Errors
///
/// A design that isn't a thermo-optic shifter's or that its 2D model refuses.
pub fn temperature(spec: &SpecIn) -> Result<Temperature, String> {
    let s = spec.spec()?;
    let device = thermo_shifter(&s)?;
    let (_, design) = s.build().map_err(text)?;
    let values = design.values();
    let get = |name: &str| {
        values
            .get(name)
            .ok_or_else(|| format!("the {} has no {name}", device.kind()))
    };
    let (design_mw, length_um, lambda_um) = (get("power")?, get("length")?, get("wavelength")?);
    let shifter: PhaseShifter = device.shifter(values).map_err(text)?;
    let rise = shifter.rise(0).map_err(text)?;
    let section = shifter.heat().section();
    let guide = &shifter.guides()[0];
    // P_π = λ / (2 s), s the index's change per W/m: the efficiency's, from the same rise
    let p_pi_mw = lambda_um * 1e-6 / (2.0 * guide.index_change(&rise).abs()) * 1e3;
    let at_pi = design_mw <= 0.0 || design_mw.is_nan();
    let power_mw = if at_pi { p_pi_mw } else { design_mw };
    let watts_per_metre = power_mw * 1e-3 / (length_um * 1e-6);
    // the window: what is drawn and the guide, with a margin, inside the heat problem's
    let (gx, gy) = (rise.grid().x().span(), rise.grid().y().span());
    let (mut x, mut y) = drawn(section, &rise).ok_or("the cross-section draws nothing")?;
    for &([cx, cy], _) in guide.weights() {
        x = [x[0].min(cx), x[1].max(cx)];
        y = [y[0].min(cy), y[1].max(cy)];
    }
    let x = [(x[0] - MARGIN).max(gx[0]), (x[1] + MARGIN).min(gx[1])];
    let y = [(y[0] - MARGIN).max(gy[0]), (y[1] + MARGIN).min(gy[1])];
    let pixel = (x[1] - x[0]).max(y[1] - y[0]) / PIXELS;
    let (nx, ny) = (
        ((x[1] - x[0]) / pixel).round().max(2.0) as usize,
        ((y[1] - y[0]) / pixel).round().max(2.0) as usize,
    );
    let mut picture = Vec::with_capacity(nx * ny);
    for j in 0..ny {
        let yc = y[0] + (j as f64 + 0.5) * (y[1] - y[0]) / ny as f64;
        for i in 0..nx {
            let xc = x[0] + (i as f64 + 0.5) * (x[1] - x[0]) / nx as f64;
            picture.push((rise.at(xc, yc) * watts_per_metre) as f32);
        }
    }
    let raster = Raster {
        nx,
        ny,
        x0: x[0],
        x1: x[1],
        y0: y[0],
        y1: y[1],
        values: picture,
    };
    let (peak, peak_at) = rise.max();
    let peak = peak * watts_per_metre;
    let isotherms = levels(peak, 8)
        .into_iter()
        .map(|level| Isoline {
            level,
            lines: contours(&raster, level),
        })
        .collect();
    let (mode, mode_step) = match weights_picture(guide.weights()) {
        Some((w, h)) => {
            let top = w.values.iter().copied().fold(0.0f32, f32::max);
            let lines = [0.5, (-2.0f64).exp()]
                .into_iter()
                .map(|share| Isoline {
                    level: share,
                    lines: contours(&w, share * f64::from(top)),
                })
                .collect();
            (lines, h)
        }
        None => (Vec::new(), f64::NAN),
    };
    let feels = guide.index_change(&rise) / guide.thermo_optic() * watts_per_metre;
    let (fine, coarse) = rise.grid().x().steps();
    let (fine_y, coarse_y) = rise.grid().y().steps();
    let grid = format!(
        "heat on {} × {} cells from {:.0} nm to {:.2} µm over {:.0} × {:.0} µm; mode on {:.0} nm \
         cells; drawn on {nx} × {ny} pixels of {:.0} nm",
        rise.grid().x().cells(),
        rise.grid().y().cells(),
        fine.min(fine_y) * 1e3,
        coarse.max(coarse_y),
        gx[1] - gx[0],
        gy[1] - gy[0],
        mode_step * 1e3,
        pixel * 1e3,
    );
    Ok(Temperature {
        outline: edges(section, x, y, nx, ny),
        raster,
        isotherms,
        mode,
        power_mw,
        at_pi,
        p_pi_mw,
        watts_per_metre,
        peak,
        peak_at,
        guide: feels,
        grid,
    })
}

/// A thermo-optic shifter's temperature at its power, by its 2D model.
#[tauri::command(async)]
pub fn device_temperature(spec: SpecIn) -> Result<Temperature, String> {
    temperature(&spec)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A picture of `f(x, y)` over [−1, 1]², `n` × `n` pixels.
    fn picture(n: usize, f: impl Fn(f64, f64) -> f64) -> Raster {
        let h = 2.0 / n as f64;
        let mut values = Vec::new();
        for j in 0..n {
            for i in 0..n {
                values.push(f(-1.0 + (i as f64 + 0.5) * h, -1.0 + (j as f64 + 0.5) * h) as f32);
            }
        }
        Raster {
            nx: n,
            ny: n,
            x0: -1.0,
            x1: 1.0,
            y0: -1.0,
            y1: 1.0,
            values,
        }
    }

    #[test]
    fn a_circle_s_contour_is_one_closed_line_on_the_circle() {
        // r², its level 0.25: the circle of radius 0.5, linear interpolation of r² between
        // centres 0.02 apart is within (0.02)²/4 of it
        let r = picture(100, |x, y| x * x + y * y);
        let lines = contours(&r, 0.25);
        assert_eq!(lines.len(), 1, "{}", lines.len());
        let line = &lines[0];
        assert_eq!(line.first(), line.last(), "closed");
        for p in line {
            let radius = p[0].hypot(p[1]);
            assert!((radius - 0.5).abs() < 1e-3, "{radius}");
        }
    }

    #[test]
    fn two_peaks_give_two_lines_and_a_saddle_none_crossed() {
        let bump = |x: f64, y: f64, cx: f64| (-((x - cx).powi(2) + y * y) / 0.05).exp();
        let r = picture(80, |x, y| bump(x, y, -0.5) + bump(x, y, 0.5));
        assert_eq!(contours(&r, 0.5).len(), 2);
        // below the saddle between them (2 e^-5 ≈ 0.013), one closed line around both
        let around = contours(&r, 0.01);
        assert_eq!(around.len(), 1);
        assert_eq!(around[0].first(), around[0].last());
        assert!(contours(&r, 3.0).is_empty());
    }

    #[test]
    fn levels_are_round_and_below_the_peak() {
        assert_eq!(levels(10.0, 5), vec![2.0, 4.0, 6.0, 8.0]);
        // 0.7 in about 8: 0.1 apart, 0.7 itself not below the peak
        let l = levels(0.7, 8);
        assert_eq!(l.len(), 6, "{l:?}");
        assert!(
            l.iter()
                .enumerate()
                .all(|(k, v)| (v - 0.1 * (k + 1) as f64).abs() < 1e-12)
        );
        assert!(levels(0.0, 8).is_empty() && levels(f64::NAN, 8).is_empty());
    }

    #[test]
    fn a_rectangle_s_edges_are_its_four_sides() {
        let mut section = Section::new("SiO2");
        section.draw(
            photonoxide::geometry::Shape::rect(
                photonoxide::geometry::Point::um(0.0, 0.0),
                photonoxide::units::Length::um(1.0),
                photonoxide::units::Length::um(0.5),
            )
            .unwrap(),
            "Si",
        );
        let lines = edges(&section, [-1.0, 1.0], [-1.0, 1.0], 40, 40);
        assert_eq!(lines.len(), 4, "{lines:?}");
        let near = |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).abs() + (a[1] - b[1]).abs() < 1e-9;
        for side in [
            [[-0.5, -0.25], [0.5, -0.25]],
            [[-0.5, 0.25], [0.5, 0.25]],
            [[-0.5, -0.25], [-0.5, 0.25]],
            [[0.5, -0.25], [0.5, 0.25]],
        ] {
            assert!(
                lines
                    .iter()
                    .any(|l| near(l[0], side[0]) && near(l[1], side[1])),
                "{side:?} in {lines:?}"
            );
        }
    }
}
