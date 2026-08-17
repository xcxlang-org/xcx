require 'json'
require 'time'

parent = { "val" => 0, "child" => nil }
t0 = Time.now
for i in 1..50
  parent = { "val" => i, "child" => parent }
end

serialized = JSON.generate(parent)
parsed = JSON.parse(serialized)

explorer = parsed
for k in 1..25
  explorer = explorer["child"]
end
val_check = explorer["val"]

t1 = Time.now
json_ms = (t1 - t0) * 1000.0
puts "json_elapsed_ms: #{'%.3f' % json_ms}"
