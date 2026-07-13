-- Benchmark: JSON field string append (100k iterations)
-- Tests pattern: data.field = data.field .. expr

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

local data = { log = "" }
local t0 = perf_us()

for i = 1, 100000 do
    data.log = data.log .. "a"
end

local t1 = perf_us()
local ms = (t1 - t0) / 1000.0
print(string.format("Field str append 100k: %.3f ms  |  len=%d", ms, #data.log))
