# T452: ARIMA order search over the seasonal orders

**Status:** done 2026-11-25 (Claude Windows "claude:gmail", statistics 43). T446's "Left": with a seasonal period,
`sheaf analyze arima --auto` and the window's "Choose p and q by AIC" treat the given seasonal P and Q as maxima, so
every P' <= P, Q' <= Q (D fixed) joins the AIC search. Not measured in Origin.
**Size:** S. **Spec:** statistics 43.

## Tests

- `crates/calc/src/arima.rs`: `auto_searches_the_seasonal_orders_up_to_the_given_ones` (a seasonal AR(1) series: the
  seasonal term is found and no searched order beats the choice; a bad period is refused).
