//! Lua scripts for performance benchmarks.
//! Used to validate that programs complete within reasonable time given the interrupt rate.

/// Benchmark: simple loop sum (1000 iterations).
/// Expected result: 500500 (1+2+...+1000).
/// With interrupt every 2 instructions, completes within ~2500–5000 ticks.
pub const BENCHMARK_LOOP: &str = r#"
local sum = 0
for i = 1, 1000 do
    sum = sum + i
end
io.write("result: " .. tostring(sum) .. "\n")
"#;
