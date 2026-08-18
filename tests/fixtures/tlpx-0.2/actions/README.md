# TL-PX 0.2 typed action fixtures

`golden.json` is the slice 3.2 cross-language contract for the distinct Submitted Intent, Authorized Action, Executed Action, and shared Action Binding preimages.

It records schema-valid source objects, exact JCS strings and UTF-8 bytes, raw SHA-256 digests, and normative `sha256:` values. The two cases distinguish required nullable payload/artifact digests from the optional, absent `retry_of_receipt_id`, and include non-ASCII text plus bound digests.

Regenerate deliberately with `npm run generate:actions:0.2` from `implementations/javascript/`, then verify with `npm run test:actions:0.2` and the Rust `action_types` test. The generator is an oracle authoring tool, not a runtime authority or PEP.
