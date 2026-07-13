-- Benchmark: General string concat in loop
local ffi = require("ffi")
ffi.cdef[[
    typedef long long LARGE_INTEGER;
    int QueryPerformanceCounter(LARGE_INTEGER *lpPerformanceCount);
    int QueryPerformanceFrequency(LARGE_INTEGER *lpFrequency);
]]

local function perf_us()
    local count = ffi.new("LARGE_INTEGER[1]")
    local freq = ffi.new("LARGE_INTEGER[1]")
    ffi.C.QueryPerformanceCounter(count)
    ffi.C.QueryPerformanceFrequency(freq)
    return tonumber(count[0]) / tonumber(freq[0]) * 1e6
end

local res = ""
local t0 = perf_us()

for i = 1, 100000 do
    res = res .. "a" .. "b"
end

local t1 = perf_us()
local ms = (t1 - t0) / 1000.0
print(string.format("General str append 100k: %.3f ms  |  len=%d", ms, #res))
