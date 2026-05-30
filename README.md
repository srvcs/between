# srvcs-between

Range orchestrator for srvcs.cloud.

**Concern:** comparison: is value within `[lo, hi]`.

`between` does no comparison of its own. It computes
`(value >= lo) AND (value <= hi)` by composing three primitives:

- `srvcs-greaterthanorequalto` answers `value >= lo` (call `{"a": value, "b": lo}`),
- `srvcs-lessthanorequalto` answers `value <= hi` (call `{"a": value, "b": hi}`),
- `srvcs-and` combines the two booleans (call `{"a": g, "b": l}`).

If any dependency is unreachable the service reports itself degraded (`503`)
rather than guessing. If a leaf dependency rejects an operand as invalid, the
`422` is forwarded unchanged.

## API

| Method | Path            | Description                                            |
| ------ | --------------- | ------------------------------------------------------ |
| GET    | `/`             | Service identity (service, concern, depends_on).       |
| POST   | `/`             | Is `value` within `[lo, hi]`? Body `{value, lo, hi}`.  |
| GET    | `/healthz`      | Liveness.                                              |
| GET    | `/readyz`       | Readiness.                                             |
| GET    | `/openapi.json` | OpenAPI document.                                      |
| GET    | `/metrics`      | Prometheus metrics.                                    |

### `POST /`

Request:

```json
{ "value": 5, "lo": 0, "hi": 10 }
```

Response:

```json
{ "value": 5, "lo": 0, "hi": 10, "result": true }
```

Statuses: `200` (computed), `422` (an operand is not a valid integer,
forwarded from a leaf dependency), `503` (a dependency is unavailable).

## Dependencies

- `srvcs-greaterthanorequalto`
- `srvcs-lessthanorequalto`
- `srvcs-and`

## Configuration

| Env var                          | Default                 | Description                              |
| -------------------------------- | ----------------------- | ---------------------------------------- |
| `SRVCS_BIND_ADDR`                | `0.0.0.0:8080`          | Host:port to bind.                       |
| `RUST_LOG`                       | `info,tower_http=info`  | Log filter.                              |
| `SRVCS_ENV`                      | `development`           | Environment label.                       |
| `SRVCS_GREATERTHANOREQUALTO_URL` | `http://127.0.0.1:8084` | Base URL of `srvcs-greaterthanorequalto`.|
| `SRVCS_LESSTHANOREQUALTO_URL`    | `http://127.0.0.1:8085` | Base URL of `srvcs-lessthanorequalto`.   |
| `SRVCS_AND_URL`                  | `http://127.0.0.1:8086` | Base URL of `srvcs-and`.                 |

## Local checks

```sh
nix flake check -L
nix develop -c sh -euc 'cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo test'
nix build .#default -L
```

The Linux container is exposed as `.#container`. On Apple Silicon, use
`linux/arm64` for the practical local check; CI builds the release image on
native `x86_64-linux`.

See [`srvcs/platform`](https://github.com/srvcs/platform) for the shared service
standard and CI workflow.
