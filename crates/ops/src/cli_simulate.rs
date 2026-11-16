//! CLI simulate arguments converted to the shared operation specification.
use crate::args::SimulateArgs;
use crate::simulate::SimulateSpec;

/// `sheaf simulate-surface` arguments as the shared specification.
pub fn simulate_surface_spec(a: &crate::args::SimulateSurfaceArgs) -> crate::simulate_surface::SimulateSurfaceOps {
    crate::simulate_surface::SimulateSurfaceOps {
        function: a.function.clone(),
        params: a.param.clone(),
        x_min: a.x_min,
        x_max: a.x_max,
        x_points: a.x_points,
        y_min: a.y_min,
        y_max: a.y_max,
        y_points: a.y_points,
        noise: a.noise,
        seed: a.seed,
        book: a.book.clone(),
    }
}

pub fn simulate_spec(a: &SimulateArgs) -> SimulateSpec {
    SimulateSpec {
        function: a.function.clone(),
        params: a.param.clone(),
        from: a.from,
        to: a.to,
        points: a.points,
        log: a.log,
        x_of: a.x_of.clone(),
        book: a.book.clone(),
        sheet: a.sheet.clone(),
        plot: a.plot.is_some() || a.graph,
    }
}
