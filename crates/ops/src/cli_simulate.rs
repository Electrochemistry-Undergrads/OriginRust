//! CLI simulate arguments converted to the shared operation specification.
use crate::args::SimulateArgs;
use crate::simulate::SimulateSpec;

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
