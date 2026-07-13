-- Benchmark: Array element string append (100k iterations)
-- Tests pattern: arr[i] = arr[i] .. expr

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

local buf = {"", "", "", "", ""}
local t0 = perf_us()

for i = 1, 100000 do
    buf[1] = buf[1] .. "a"
end

local t1 = perf_us()
local ms = (t1 - t0) / 1000.0
print(string.format("Array elem str append 100k: %.3f ms  |  len=%d", ms, #buf[1]))
