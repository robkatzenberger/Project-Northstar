# TL-PX 0.2 policy bundle fixture

`manifest-golden.json` pins the slice 2.4 policy-bundle manifest and its domain-separated Northstar JCS hash. The JavaScript contract oracle and Rust hash implementation must produce the exact `policy_bundle_hash` shown in the fixture.

The fixture validates a policy manifest and digest, not a 0.2 runtime or policy language. `content_hash` binds the separately retained activated policy content in the format named by `content_type`.
