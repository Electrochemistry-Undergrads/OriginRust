//! Baseline subtraction. Constants and straight lines are closed form; the curves through anchor
//! points reuse the interpolation numbers from Origin 2016 (`tools/origin-ref/interp-suite1.labtalk.txt`,
//! columns `lin_extrap`, `spl_notaknot_extrap`, `aki_extrap`), because the baseline is the interpolated
//! curve through the anchors. Tolerance: relative 1e-10.

use sheaf_calc::{BaselineMethod, InterpMethod, SplineBoundary, subtract_baseline, subtract_constant, subtract_line};
#[allow(dead_code)]
fn grid(n: usize, start: f64, step: f64) -> Vec<f64> {
    (0..n).map(|i| start + i as f64 * step).collect()
}

#[allow(dead_code)]
fn close(name: &str, got: f64, want: f64, tol: f64) {
    if want.is_nan() {
        assert!(got.is_nan(), "{name}: got {got:?}, Origin gave NaN");
        return;
    }
    let ok = (got - want).abs() <= tol * want.abs().max(1.0);
    assert!(ok, "{name}: got {got:?}, Origin gave {want:?}");
}

#[allow(dead_code)]
fn check(name: &str, got: &[f64], samples: &[(usize, f64)], tol: f64) {
    for &(i, w) in samples {
        close(&format!("{name}[{i}]"), got[i], w, tol);
    }
}

fn anchors() -> (Vec<f64>, Vec<f64>) {
    let x: Vec<f64> = (0..11).map(|i| 10.0 * (i as f64 / 10.0).powf(1.3)).collect();
    let y = x.iter().map(|x| x.sin() + 0.1 * x).collect();
    (x, y)
}

#[test]
fn constant_and_line_are_closed_form() {
    let x = [0.0, 1.0, 2.0, 3.0];
    let y = [5.0, 4.0, 9.0, 7.0];
    assert_eq!(subtract_constant(&y, 2.0), vec![3.0, 2.0, 7.0, 5.0]);
    // line through (0, 1) and (2, 5): 1 + 2 x
    let r = subtract_line(&x, &y, (0.0, 1.0), (2.0, 5.0));
    assert_eq!(r, vec![4.0, 1.0, 4.0, 0.0]);
    let (c, b) = subtract_baseline(&x, &y, &BaselineMethod::Constant(1.5));
    assert_eq!(b, vec![1.5; 4]);
    assert_eq!(c, vec![3.5, 2.5, 7.5, 5.5]);
}

#[test]
fn anchors_follow_the_interpolation_tools() {
    let (ax, ay) = anchors();
    let at = grid(33, -1.0, 0.37);
    let points: Vec<(f64, f64)> = ax.iter().cloned().zip(ay.iter().cloned()).collect();
    let cases: Vec<(&str, InterpMethod, &[(usize, f64)])> = vec![
        (
            "linear",
            InterpMethod::Linear,
            &[
                (0, -1.0586578902449784),
                (1, -0.6669544708543363),
                (2, -0.2752510514636944),
                (3, 0.11645236792694748),
                (4, 0.5081557873175895),
                (5, 0.7860086323803164),
                (6, 1.0569460805613038),
                (7, 1.0713012653601064),
                (8, 1.0755153949862595),
                (9, 0.9076868782639149),
                (10, 0.6460750210602231),
                (11, 0.3822282884945476),
                (12, 0.09425574364249112),
                (13, -0.19371680120956597),
                (14, -0.3895807420649442),
                (15, -0.39053330805930464),
                (16, -0.3914858740536651),
                (17, -0.2638778703655434),
                (18, 0.06898345750991552),
                (19, 0.4018447853853744),
                (20, 0.7320885443089443),
                (21, 1.056172126712),
                (22, 1.3802557091150571),
                (23, 1.6761845507544813),
                (24, 1.6283882655004427),
                (25, 1.5805919802464041),
                (26, 1.5327956949923658),
                (27, 1.2954480098154066),
                (28, 0.9879197180720732),
                (29, 0.6803914263287385),
                (30, 0.3728631345854051),
                (31, 0.0653348428420703),
                (32, -0.24219344890126304),
            ],
        ),
        (
            "cubic spline",
            InterpMethod::CubicSpline,
            &[
                (0, -1.0526624568746374),
                (1, -0.6957961820720178),
                (2, -0.2920606579018381),
                (3, 0.12202057694737616),
                (4, 0.5099239837871001),
                (5, 0.8351260239288081),
                (6, 1.0611031586839745),
                (7, 1.1574042204751276),
                (8, 1.1208048392585224),
                (9, 0.9574775879508665),
                (10, 0.6956865665201575),
                (11, 0.3787073788506646),
                (12, 0.05121121616413182),
                (13, -0.23868493415670142),
                (14, -0.4424327714301364),
                (15, -0.5269960135043462),
                (16, -0.48419442916799144),
                (17, -0.3085605771854226),
                (18, -0.014900602490585751),
                (19, 0.3545906953042315),
                (20, 0.7557660692248607),
                (21, 1.1435230502742313),
                (22, 1.4711321399410289),
                (23, 1.6917102192736955),
                (24, 1.7726212584750056),
                (25, 1.7266966054011146),
                (26, 1.5758181238977016),
                (27, 1.3418676778104446),
                (28, 1.0467271309850252),
                (29, 0.7122783472671201),
                (30, 0.3604031905024119),
                (31, 0.012983524536576661),
                (32, -0.308098786784703),
            ],
        ),
        (
            "akima",
            InterpMethod::Akima,
            &[
                (0, -1.4557019678264154),
                (1, -0.8554403868431075),
                (2, -0.3275952679816652),
                (3, 0.1278333887579115),
                (4, 0.510845583375623),
                (5, 0.8241691442295418),
                (6, 1.060044892985526),
                (7, 1.1880811892793088),
                (8, 1.145188729027662),
                (9, 0.91787994844386),
                (10, 0.6558927853814944),
                (11, 0.38326551978533613),
                (12, 0.0943014759418937),
                (13, -0.20011084570972798),
                (14, -0.4674253717645413),
                (15, -0.6027422714597328),
                (16, -0.5430414684106512),
                (17, -0.2666257229586049),
                (18, 0.06623604509856512),
                (19, 0.4023632380049029),
                (20, 0.7335288793011854),
                (21, 1.0621100156559948),
                (22, 1.3868454133583619),
                (23, 1.7024141660738157),
                (24, 1.8364937168934043),
                (25, 1.750627236929976),
                (26, 1.5691678086367944),
                (27, 1.3714789747930867),
                (28, 1.1021135076319155),
                (29, 0.7564173268203767),
                (30, 0.33439043235847343),
                (31, -0.16396717575379816),
                (32, -0.7386554975164334),
            ],
        ),
    ];
    let signal: Vec<f64> = at.iter().map(|x| 0.3 * x + 1.0).collect();
    for (name, method, want) in cases {
        let m = BaselineMethod::Anchors { points: points.clone(), method, boundary: SplineBoundary::NotAKnot };
        let (corrected, base) = subtract_baseline(&at, &signal, &m);
        check(name, &base, want, 1e-10);
        for i in 0..at.len() {
            close(&format!("{name} corrected[{i}]"), corrected[i], signal[i] - base[i], 1e-14);
        }
    }
}
