//! Peak search options of Origin `pkfind` (local maximum method): smoothing before the search, feet of
//! the peaks (`oleft`, `oright`), the foot threshold `fheight` and the largest foot distance `hwidth`.
//!
//! Reference scripts: `tools/origin-ref/pkfind-feet-suite1.lines.txt` and `pkfind-feet-suite2.lines.txt` (run with
//! `run-labtalk-safe.ps1`; x = 0..20 step 0.1, 201 points). "clean" is the sum of four gaussians (one negative)
//! plus 0.2; "ripple" adds 0.12 sin(9x). Compared: the 1-based rows of the peaks and of both feet (exact). The one
//! case with `hwidth` 0.2 is left out: the limit lies exactly on a grid distance and Origin's rounding decides
//! per peak whether the third point is searched. What the runs showed is in `docs/spec/analysis-tools.md`, section 7.

use sheaf_calc::{PeakDirection, PeakFilter, PeakOptions, find_peaks};

fn data(kind: &str) -> (Vec<f64>, Vec<f64>) {
    let x: Vec<f64> = (0..201).map(|i| i as f64 * 0.1).collect();
    let y = x
        .iter()
        .map(|&x| {
            let clean = (-(x - 4.0).powi(2)).exp() + 0.7 * (-(x - 9.0).powi(2) / 2.0).exp()
                - 0.6 * (-(x - 13.0).powi(2) / 0.5).exp()
                + 0.8 * (-(x - 17.0).powi(2)).exp()
                + 0.2;
            if kind == "ripple" { clean + 0.12 * (9.0 * x).sin() } else { clean }
        })
        .collect();
    (x, y)
}

type Case = (&'static str, PeakOptions, &'static [usize], &'static [usize], &'static [usize]);

#[test]
fn peaks_and_feet_match_origin() {
    let cases: Vec<(&str, Vec<Case>)> = vec![
        (
            "clean",
            vec![
                (
                    "defaults",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 0,
                        foot_height: 0.0,
                        max_half_width: None,
                    },
                    &[41, 91, 131, 171],
                    &[1, 63, 124, 138],
                    &[63, 124, 138, 201],
                ),
                (
                    "hwidth=1",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 0,
                        foot_height: 0.0,
                        max_half_width: Some(1.0),
                    },
                    &[41, 91, 131, 171],
                    &[31, 81, 124, 161],
                    &[51, 101, 138, 181],
                ),
                (
                    "defaults",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 0,
                        foot_height: 0.0,
                        max_half_width: None,
                    },
                    &[41, 91, 131, 171],
                    &[1, 63, 124, 138],
                    &[63, 124, 138, 201],
                ),
                (
                    "hwidth=0",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 0,
                        foot_height: 0.0,
                        max_half_width: Some(0.0),
                    },
                    &[41, 91, 131, 171],
                    &[39, 89, 129, 169],
                    &[43, 93, 133, 173],
                ),
                (
                    "hwidth=5",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 0,
                        foot_height: 0.0,
                        max_half_width: Some(5.0),
                    },
                    &[41, 91, 131, 171],
                    &[1, 63, 124, 138],
                    &[63, 124, 138, 201],
                ),
                (
                    "hwidth=0",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 0,
                        foot_height: 0.0,
                        max_half_width: Some(0.0),
                    },
                    &[41, 91, 131, 171],
                    &[39, 89, 129, 169],
                    &[43, 93, 133, 173],
                ),
                (
                    "fheight=0.1",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 0,
                        foot_height: 0.1,
                        max_half_width: None,
                    },
                    &[41, 91, 131, 171],
                    &[1, 63, 125, 140],
                    &[63, 122, 137, 201],
                ),
                (
                    "fheight=0.1 hwidth=5",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 0,
                        foot_height: 0.1,
                        max_half_width: Some(5.0),
                    },
                    &[41, 91, 131, 171],
                    &[1, 63, 125, 140],
                    &[63, 122, 137, 201],
                ),
                (
                    "fheight=0.0",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 0,
                        foot_height: 0.0,
                        max_half_width: None,
                    },
                    &[41, 91, 131, 171],
                    &[1, 63, 124, 138],
                    &[63, 124, 138, 201],
                ),
                (
                    "fheight=0.5 hwidth=5",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 0,
                        foot_height: 0.5,
                        max_half_width: Some(5.0),
                    },
                    &[41, 91, 131, 171],
                    &[30, 77, 130, 161],
                    &[52, 105, 132, 181],
                ),
                (
                    "fheight=0.05 hwidth=5",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 0,
                        foot_height: 0.05,
                        max_half_width: Some(5.0),
                    },
                    &[41, 91, 131, 171],
                    &[1, 63, 124, 139],
                    &[63, 123, 138, 201],
                ),
                (
                    "fheight=0.5 hwidth=0",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 0,
                        foot_height: 0.5,
                        max_half_width: Some(0.0),
                    },
                    &[41, 91, 131, 171],
                    &[39, 89, 130, 169],
                    &[43, 93, 132, 173],
                ),
                (
                    "dir=p",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Positive,
                        filter: PeakFilter::None,
                        smooth_points: 0,
                        foot_height: 0.0,
                        max_half_width: None,
                    },
                    &[41, 91, 171],
                    &[1, 63, 124],
                    &[63, 124, 201],
                ),
                (
                    "dir=n",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Negative,
                        filter: PeakFilter::None,
                        smooth_points: 0,
                        foot_height: 0.0,
                        max_half_width: None,
                    },
                    &[131],
                    &[124],
                    &[138],
                ),
                (
                    "filter=h value=0.5",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::Height(0.5),
                        smooth_points: 0,
                        foot_height: 0.0,
                        max_half_width: None,
                    },
                    &[41, 91, 171],
                    &[1, 63, 124],
                    &[63, 124, 201],
                ),
                (
                    "filter=num value=3",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::Number(3),
                        smooth_points: 0,
                        foot_height: 0.0,
                        max_half_width: None,
                    },
                    &[41, 91, 171],
                    &[1, 63, 124],
                    &[63, 124, 201],
                ),
                (
                    "filter=h value=0.5 dir=p",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Positive,
                        filter: PeakFilter::Height(0.5),
                        smooth_points: 0,
                        foot_height: 0.0,
                        max_half_width: None,
                    },
                    &[41, 91, 171],
                    &[1, 63, 124],
                    &[63, 124, 201],
                ),
                (
                    "hwidth=0.05",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 0,
                        foot_height: 0.0,
                        max_half_width: Some(0.05),
                    },
                    &[41, 91, 131, 171],
                    &[39, 89, 129, 169],
                    &[43, 93, 133, 173],
                ),
                (
                    "hwidth=0.1",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 0,
                        foot_height: 0.0,
                        max_half_width: Some(0.1),
                    },
                    &[41, 91, 131, 171],
                    &[39, 89, 129, 169],
                    &[43, 93, 133, 173],
                ),
                (
                    "hwidth=0.15",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 0,
                        foot_height: 0.0,
                        max_half_width: Some(0.15),
                    },
                    &[41, 91, 131, 171],
                    &[39, 89, 129, 169],
                    &[43, 93, 133, 173],
                ),
                (
                    "hwidth=0.25",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 0,
                        foot_height: 0.0,
                        max_half_width: Some(0.25),
                    },
                    &[41, 91, 131, 171],
                    &[38, 88, 128, 168],
                    &[44, 94, 134, 174],
                ),
                (
                    "hwidth=0 npts=3",
                    PeakOptions {
                        npts: 3,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 0,
                        foot_height: 0.0,
                        max_half_width: Some(0.0),
                    },
                    &[41, 91, 131, 171],
                    &[39, 89, 129, 169],
                    &[43, 93, 133, 173],
                ),
                (
                    "hwidth=0.1 npts=3",
                    PeakOptions {
                        npts: 3,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 0,
                        foot_height: 0.0,
                        max_half_width: Some(0.1),
                    },
                    &[41, 91, 131, 171],
                    &[39, 89, 129, 169],
                    &[43, 93, 133, 173],
                ),
                (
                    "hwidth=0.35 npts=3",
                    PeakOptions {
                        npts: 3,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 0,
                        foot_height: 0.0,
                        max_half_width: Some(0.35),
                    },
                    &[41, 91, 131, 171],
                    &[37, 87, 127, 167],
                    &[45, 95, 135, 175],
                ),
                (
                    "smooth=2",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 2,
                        foot_height: 0.0,
                        max_half_width: None,
                    },
                    &[41, 91, 131, 171],
                    &[1, 63, 124, 138],
                    &[63, 124, 138, 201],
                ),
                (
                    "smooth=3",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 3,
                        foot_height: 0.0,
                        max_half_width: None,
                    },
                    &[41, 91, 131, 171],
                    &[1, 63, 124, 138],
                    &[63, 124, 138, 201],
                ),
                (
                    "smooth=4",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 4,
                        foot_height: 0.0,
                        max_half_width: None,
                    },
                    &[41, 91, 131, 171],
                    &[1, 63, 124, 138],
                    &[63, 124, 138, 201],
                ),
                (
                    "smooth=5",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 5,
                        foot_height: 0.0,
                        max_half_width: None,
                    },
                    &[41, 91, 131, 171],
                    &[1, 63, 124, 138],
                    &[63, 124, 138, 201],
                ),
                (
                    "smooth=7",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 7,
                        foot_height: 0.0,
                        max_half_width: None,
                    },
                    &[41, 91, 131, 171],
                    &[1, 63, 124, 138],
                    &[63, 124, 138, 201],
                ),
                (
                    "smooth=9",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 9,
                        foot_height: 0.0,
                        max_half_width: None,
                    },
                    &[41, 91, 131, 171],
                    &[1, 63, 124, 138],
                    &[63, 124, 138, 201],
                ),
                (
                    "smooth=11",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 11,
                        foot_height: 0.0,
                        max_half_width: None,
                    },
                    &[41, 91, 131, 171],
                    &[1, 63, 124, 139],
                    &[63, 124, 139, 201],
                ),
                (
                    "smooth=15",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 15,
                        foot_height: 0.0,
                        max_half_width: None,
                    },
                    &[41, 91, 131, 171],
                    &[10, 63, 123, 139],
                    &[63, 123, 139, 201],
                ),
                (
                    "smooth=25",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 25,
                        foot_height: 0.0,
                        max_half_width: None,
                    },
                    &[41, 91, 131, 171],
                    &[16, 63, 123, 140],
                    &[63, 123, 140, 196],
                ),
                (
                    "smooth=5 fheight=0.3",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 5,
                        foot_height: 0.3,
                        max_half_width: None,
                    },
                    &[41, 91, 131, 171],
                    &[25, 71, 127, 156],
                    &[57, 111, 135, 186],
                ),
            ],
        ),
        (
            "ripple",
            vec![
                (
                    "dir=p",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Positive,
                        filter: PeakFilter::None,
                        smooth_points: 0,
                        foot_height: 0.0,
                        max_half_width: None,
                    },
                    &[
                        3, 10, 17, 24, 38, 44, 51, 58, 66, 73, 80, 87, 93, 100, 107, 114, 121, 143, 149, 157, 164, 170,
                        177, 184, 191, 198,
                    ],
                    &[
                        1, 6, 13, 20, 27, 41, 49, 55, 62, 69, 76, 83, 90, 97, 104, 111, 118, 140, 146, 153, 159, 166,
                        174, 181, 188, 195,
                    ],
                    &[
                        6, 13, 20, 27, 41, 49, 55, 62, 69, 76, 83, 90, 97, 104, 111, 118, 140, 146, 153, 159, 166, 174,
                        181, 188, 195, 201,
                    ],
                ),
                (
                    "dir=n",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Negative,
                        filter: PeakFilter::None,
                        smooth_points: 0,
                        foot_height: 0.0,
                        max_half_width: None,
                    },
                    &[126, 132, 138],
                    &[123, 128, 136],
                    &[128, 136, 140],
                ),
                (
                    "filter=h value=0.5",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::Height(0.5),
                        smooth_points: 0,
                        foot_height: 0.0,
                        max_half_width: None,
                    },
                    &[38, 44, 51, 80, 87, 93, 100, 107, 132, 164, 170, 177],
                    &[13, 41, 49, 62, 83, 90, 97, 104, 123, 140, 166, 174],
                    &[41, 49, 62, 83, 90, 97, 104, 123, 140, 166, 174, 195],
                ),
                (
                    "filter=num value=3",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::Number(3),
                        smooth_points: 0,
                        foot_height: 0.0,
                        max_half_width: None,
                    },
                    &[38, 44, 170],
                    &[13, 41, 140],
                    &[41, 140, 195],
                ),
                (
                    "filter=h value=0.5 dir=p",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Positive,
                        filter: PeakFilter::Height(0.5),
                        smooth_points: 0,
                        foot_height: 0.0,
                        max_half_width: None,
                    },
                    &[38, 44, 51, 80, 87, 93, 100, 107, 164, 170, 177],
                    &[13, 41, 49, 62, 83, 90, 97, 104, 140, 166, 174],
                    &[41, 49, 62, 83, 90, 97, 104, 140, 166, 174, 195],
                ),
                (
                    "hwidth=0.05",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 0,
                        foot_height: 0.0,
                        max_half_width: Some(0.05),
                    },
                    &[
                        3, 10, 17, 24, 38, 44, 51, 58, 66, 73, 80, 87, 93, 100, 107, 114, 121, 126, 132, 138, 143, 149,
                        157, 164, 170, 177, 184, 191, 198,
                    ],
                    &[
                        1, 8, 15, 22, 36, 42, 49, 56, 64, 71, 78, 85, 91, 98, 105, 112, 119, 124, 130, 136, 141, 147,
                        155, 162, 168, 175, 182, 189, 196,
                    ],
                    &[
                        5, 12, 19, 26, 40, 46, 53, 60, 68, 75, 82, 89, 95, 102, 109, 116, 123, 128, 134, 140, 145, 151,
                        159, 166, 172, 179, 186, 193, 200,
                    ],
                ),
                (
                    "hwidth=0.1",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 0,
                        foot_height: 0.0,
                        max_half_width: Some(0.1),
                    },
                    &[
                        3, 10, 17, 24, 38, 44, 51, 58, 66, 73, 80, 87, 93, 100, 107, 114, 121, 126, 132, 138, 143, 149,
                        157, 164, 170, 177, 184, 191, 198,
                    ],
                    &[
                        1, 8, 15, 22, 36, 42, 49, 56, 64, 71, 78, 85, 91, 98, 105, 112, 119, 124, 130, 136, 141, 147,
                        155, 162, 168, 175, 182, 189, 196,
                    ],
                    &[
                        5, 12, 19, 26, 40, 46, 53, 60, 68, 75, 82, 89, 95, 102, 109, 116, 123, 128, 134, 140, 145, 151,
                        159, 166, 172, 179, 186, 193, 200,
                    ],
                ),
                (
                    "hwidth=0.15",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 0,
                        foot_height: 0.0,
                        max_half_width: Some(0.15),
                    },
                    &[
                        3, 10, 17, 24, 38, 44, 51, 58, 66, 73, 80, 87, 93, 100, 107, 114, 121, 126, 132, 138, 143, 149,
                        157, 164, 170, 177, 184, 191, 198,
                    ],
                    &[
                        1, 8, 15, 22, 36, 42, 49, 56, 64, 71, 78, 85, 91, 98, 105, 112, 119, 124, 130, 136, 141, 147,
                        155, 162, 168, 175, 182, 189, 196,
                    ],
                    &[
                        5, 12, 19, 26, 40, 46, 53, 60, 68, 75, 82, 89, 95, 102, 109, 116, 123, 128, 134, 140, 145, 151,
                        159, 166, 172, 179, 186, 193, 200,
                    ],
                ),
                (
                    "hwidth=0.25",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 0,
                        foot_height: 0.0,
                        max_half_width: Some(0.25),
                    },
                    &[
                        3, 10, 17, 24, 38, 44, 51, 58, 66, 73, 80, 87, 93, 100, 107, 114, 121, 126, 132, 138, 143, 149,
                        157, 164, 170, 177, 184, 191, 198,
                    ],
                    &[
                        1, 7, 14, 21, 35, 41, 49, 55, 63, 70, 77, 84, 90, 97, 104, 111, 118, 123, 129, 136, 140, 146,
                        154, 161, 167, 174, 181, 188, 195,
                    ],
                    &[
                        6, 13, 20, 27, 41, 47, 54, 61, 69, 76, 83, 90, 96, 103, 110, 117, 123, 128, 135, 140, 146, 152,
                        159, 166, 173, 180, 187, 194, 201,
                    ],
                ),
                (
                    "hwidth=0 npts=3",
                    PeakOptions {
                        npts: 3,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 0,
                        foot_height: 0.0,
                        max_half_width: Some(0.0),
                    },
                    &[
                        10, 17, 24, 38, 44, 58, 66, 73, 80, 87, 93, 100, 107, 114, 121, 132, 143, 149, 157, 164, 170,
                        177, 184, 191, 198,
                    ],
                    &[
                        8, 15, 22, 36, 42, 56, 64, 71, 78, 85, 91, 98, 105, 112, 119, 130, 141, 147, 155, 162, 168,
                        175, 182, 189, 196,
                    ],
                    &[
                        12, 19, 26, 40, 46, 60, 68, 75, 82, 89, 95, 102, 109, 116, 123, 134, 145, 151, 159, 166, 172,
                        179, 186, 193, 200,
                    ],
                ),
                (
                    "hwidth=0.1 npts=3",
                    PeakOptions {
                        npts: 3,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 0,
                        foot_height: 0.0,
                        max_half_width: Some(0.1),
                    },
                    &[
                        10, 17, 24, 38, 44, 58, 66, 73, 80, 87, 93, 100, 107, 114, 121, 132, 143, 149, 157, 164, 170,
                        177, 184, 191, 198,
                    ],
                    &[
                        8, 15, 22, 36, 42, 56, 64, 71, 78, 85, 91, 98, 105, 112, 119, 130, 141, 147, 155, 162, 168,
                        175, 182, 189, 196,
                    ],
                    &[
                        12, 19, 26, 40, 46, 60, 68, 75, 82, 89, 95, 102, 109, 116, 123, 134, 145, 151, 159, 166, 172,
                        179, 186, 193, 200,
                    ],
                ),
                (
                    "hwidth=0.35 npts=3",
                    PeakOptions {
                        npts: 3,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 0,
                        foot_height: 0.0,
                        max_half_width: Some(0.35),
                    },
                    &[
                        10, 17, 24, 38, 44, 58, 66, 73, 80, 87, 93, 100, 107, 114, 121, 132, 143, 149, 157, 164, 170,
                        177, 184, 191, 198,
                    ],
                    &[
                        6, 13, 20, 34, 41, 55, 62, 69, 76, 83, 90, 97, 104, 111, 118, 128, 140, 146, 153, 160, 166,
                        174, 181, 188, 195,
                    ],
                    &[
                        13, 20, 27, 41, 48, 62, 69, 76, 83, 90, 97, 104, 111, 118, 123, 136, 146, 153, 159, 166, 174,
                        181, 188, 195, 201,
                    ],
                ),
                (
                    "smooth=2",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 2,
                        foot_height: 0.0,
                        max_half_width: None,
                    },
                    &[
                        3, 10, 17, 24, 38, 44, 51, 58, 66, 73, 80, 87, 93, 100, 107, 114, 121, 126, 132, 138, 143, 149,
                        157, 164, 170, 177, 184, 191, 198,
                    ],
                    &[
                        1, 6, 13, 20, 27, 41, 49, 55, 62, 69, 76, 83, 90, 97, 104, 111, 118, 123, 128, 136, 140, 146,
                        153, 159, 166, 174, 181, 188, 195,
                    ],
                    &[
                        6, 13, 20, 27, 41, 49, 55, 62, 69, 76, 83, 90, 97, 104, 111, 118, 123, 128, 136, 140, 146, 153,
                        159, 166, 174, 181, 188, 195, 201,
                    ],
                ),
                (
                    "smooth=3",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 3,
                        foot_height: 0.0,
                        max_half_width: None,
                    },
                    &[
                        3, 10, 17, 24, 38, 44, 51, 58, 66, 73, 80, 87, 93, 100, 107, 114, 121, 126, 132, 138, 143, 149,
                        157, 164, 170, 177, 184, 191, 198,
                    ],
                    &[
                        1, 6, 13, 20, 27, 41, 49, 55, 62, 69, 76, 83, 90, 97, 104, 111, 118, 123, 128, 136, 140, 146,
                        153, 159, 166, 174, 181, 188, 195,
                    ],
                    &[
                        6, 13, 20, 27, 41, 49, 55, 62, 69, 76, 83, 90, 97, 104, 111, 118, 123, 128, 136, 140, 146, 153,
                        159, 166, 174, 181, 188, 195, 201,
                    ],
                ),
                (
                    "smooth=4",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 4,
                        foot_height: 0.0,
                        max_half_width: None,
                    },
                    &[
                        3, 10, 17, 24, 38, 44, 51, 58, 66, 73, 80, 87, 93, 100, 107, 114, 121, 126, 132, 138, 143, 149,
                        157, 164, 170, 176, 184, 191, 198,
                    ],
                    &[
                        1, 6, 13, 20, 27, 41, 49, 56, 62, 69, 76, 83, 90, 97, 104, 111, 118, 123, 128, 136, 140, 146,
                        153, 159, 166, 174, 181, 188, 195,
                    ],
                    &[
                        6, 13, 20, 27, 41, 49, 56, 62, 69, 76, 83, 90, 97, 104, 111, 118, 123, 128, 136, 140, 146, 153,
                        159, 166, 174, 181, 188, 195, 201,
                    ],
                ),
                (
                    "smooth=5",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 5,
                        foot_height: 0.0,
                        max_half_width: None,
                    },
                    &[
                        3, 10, 17, 24, 38, 44, 51, 58, 66, 73, 80, 87, 93, 100, 107, 114, 121, 126, 132, 138, 143, 149,
                        157, 164, 170, 176, 184, 191, 198,
                    ],
                    &[
                        1, 6, 13, 20, 27, 41, 49, 56, 62, 69, 76, 83, 90, 97, 104, 111, 118, 123, 128, 136, 140, 146,
                        153, 159, 166, 174, 181, 188, 195,
                    ],
                    &[
                        6, 13, 20, 27, 41, 49, 56, 62, 69, 76, 83, 90, 97, 104, 111, 118, 123, 128, 136, 140, 146, 153,
                        159, 166, 174, 181, 188, 195, 201,
                    ],
                ),
                (
                    "smooth=7",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 7,
                        foot_height: 0.0,
                        max_half_width: None,
                    },
                    &[
                        3, 10, 17, 24, 38, 44, 58, 66, 73, 80, 87, 93, 100, 107, 114, 121, 126, 132, 138, 143, 149,
                        157, 164, 170, 176, 184, 191, 198,
                    ],
                    &[
                        1, 6, 13, 20, 27, 41, 56, 62, 69, 76, 82, 90, 98, 105, 111, 118, 123, 127, 137, 140, 146, 153,
                        159, 166, 174, 182, 188, 195,
                    ],
                    &[
                        6, 13, 20, 27, 41, 56, 62, 69, 76, 82, 90, 98, 105, 111, 118, 123, 127, 137, 140, 146, 153,
                        159, 166, 174, 182, 188, 195, 201,
                    ],
                ),
                (
                    "smooth=9",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 9,
                        foot_height: 0.0,
                        max_half_width: None,
                    },
                    &[3, 10, 17, 24, 39, 43, 58, 66, 73, 87, 93, 100, 107, 114, 121, 132, 143, 149, 157, 170, 191, 198],
                    &[1, 6, 13, 20, 26, 41, 56, 62, 69, 75, 90, 98, 105, 111, 118, 123, 139, 146, 153, 159, 188, 195],
                    &[6, 13, 20, 26, 41, 56, 62, 69, 75, 90, 98, 105, 111, 118, 123, 139, 146, 153, 159, 188, 195, 201],
                ),
                (
                    "smooth=11",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 11,
                        foot_height: 0.0,
                        max_half_width: None,
                    },
                    &[10, 17, 40, 66, 88, 93, 114, 131, 143, 150, 171, 191, 199],
                    &[6, 13, 20, 62, 68, 89, 112, 123, 139, 146, 152, 189, 195],
                    &[13, 20, 62, 68, 89, 112, 123, 139, 146, 152, 189, 195, 201],
                ),
                (
                    "smooth=15",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 15,
                        foot_height: 0.0,
                        max_half_width: None,
                    },
                    &[6, 13, 20, 41, 62, 70, 90, 117, 130, 146, 153, 173, 195],
                    &[1, 10, 17, 23, 59, 65, 71, 115, 124, 138, 149, 155, 191],
                    &[10, 17, 23, 59, 65, 71, 115, 124, 138, 149, 155, 191, 201],
                ),
                (
                    "smooth=25",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 25,
                        foot_height: 0.0,
                        max_half_width: None,
                    },
                    &[8, 17, 41, 92, 131, 150, 171, 192],
                    &[1, 13, 20, 62, 123, 140, 152, 189],
                    &[13, 20, 62, 123, 140, 152, 189, 201],
                ),
                (
                    "smooth=5 fheight=0.3",
                    PeakOptions {
                        npts: 2,
                        direction: PeakDirection::Both,
                        filter: PeakFilter::None,
                        smooth_points: 5,
                        foot_height: 0.3,
                        max_half_width: None,
                    },
                    &[
                        3, 10, 17, 24, 38, 44, 51, 58, 66, 73, 80, 87, 93, 100, 107, 114, 121, 126, 132, 138, 143, 149,
                        157, 164, 170, 176, 184, 191, 198,
                    ],
                    &[
                        2, 9, 16, 22, 27, 41, 49, 57, 64, 70, 76, 83, 90, 97, 104, 113, 120, 125, 129, 137, 142, 148,
                        154, 159, 166, 174, 181, 190, 197,
                    ],
                    &[
                        4, 11, 18, 26, 41, 49, 55, 60, 67, 76, 83, 90, 97, 104, 110, 116, 122, 127, 134, 139, 144, 151,
                        159, 166, 174, 181, 186, 192, 199,
                    ],
                ),
            ],
        ),
    ];
    for (kind, list) in cases {
        let (x, y) = data(kind);
        for (name, opts, centres, left, right) in list {
            let got = find_peaks(&x, &y, &opts);
            assert_eq!(got.rows, centres, "{kind} {name}: peaks");
            assert_eq!(got.left, left, "{kind} {name}: left feet");
            assert_eq!(got.right, right, "{kind} {name}: right feet");
        }
    }
}
