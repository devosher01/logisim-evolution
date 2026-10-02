# Netlist Compiler

`netlist-compiler` transforms a validated document into deterministic
connectivity. It does not mutate the document and it has no simulator or UI
dependencies.

Every declared port receives exactly one net, including unconnected ports.
Explicit connections are merged with union-find. Net ordering is deterministic
so compiled output can be compared, cached, serialized, and replayed.

The next extensions are width-aware bus segments, junction diagnostics, and
incremental recompilation of only affected connected components.
