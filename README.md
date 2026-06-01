# srvcs-between

## Name

| Field | Value |
| --- | --- |
| Service | `srvcs-between` |
| Slug | `between` |
| Repository | `srvcs/between` |
| Package | `srvcs-between` |
| Kind | `orchestrator` |

## Function

comparison: is value within [lo, hi]

## Dependencies

| Dependency | Repository |
| --- | --- |
| `srvcs-greaterthanorequalto` | [srvcs/greaterthanorequalto](https://github.com/srvcs/greaterthanorequalto) |
| `srvcs-lessthanorequalto` | [srvcs/lessthanorequalto](https://github.com/srvcs/lessthanorequalto) |
| `srvcs-and` | [srvcs/and](https://github.com/srvcs/and) |

## API

| Method | Path | Purpose |
| --- | --- | --- |
| `GET` | `/` | Service identity |
| `POST` | `/` | Evaluate the service function |
| `GET` | `/healthz` | Liveness probe |
| `GET` | `/readyz` | Readiness probe |
| `GET` | `/metrics` | Prometheus metrics |
| `GET` | `/openapi.json` | OpenAPI document |

## Inputs

| Name | Type | Required |
| --- | --- | --- |
| `value` | `json` | yes |
| `lo` | `json` | yes |
| `hi` | `json` | yes |

## Outputs

| Name | Type |
| --- | --- |
| `value` | `json` |
| `lo` | `json` |
| `hi` | `json` |
| `result` | `boolean` |

## Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `SRVCS_BIND_ADDR` | `0.0.0.0:8080` | Bind address |
| `SRVCS_ENV` | `development` | Environment label for logs |
| `RUST_LOG` | `info,tower_http=info` | Tracing filter |
| `SRVCS_AND_URL` | `http://127.0.0.1:8086` | Base URL for srvcs-and |
| `SRVCS_GREATERTHANOREQUALTO_URL` | `` | Base URL for srvcs-greaterthanorequalto |
| `SRVCS_LESSTHANOREQUALTO_URL` | `` | Base URL for srvcs-lessthanorequalto |

## Error Behavior

- `422` means the request could not be evaluated for the documented input shape.
- `503` means a required dependency was unavailable or returned an unexpected response.
- Dependency validation errors are forwarded when this service delegates validation.

## Local Checks

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

See the [srvcs service standard](https://github.com/srvcs/platform/blob/main/STANDARD.md) for the full operational contract.

## Metadata

Machine-readable service metadata lives in `srvcs.yaml`. Keep it aligned with this README when the service contract changes.
