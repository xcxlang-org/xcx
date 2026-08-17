require 'json'
require 'time'

payload_tbl = []
50000.times do |i|
  payload_tbl << { id: i, val: i * 2, active: true }
end

t0 = Time.now
arr = JSON.parse(JSON.generate(payload_tbl))
filtered_arr = []
arr.each_with_index do |item, i|
  val = item["val"]
  if val % 4 == 0
    item_map = { val: val }
    map_json = JSON.parse(JSON.generate(item_map))
    map_json["id"] = i
    filtered_arr << map_json
  end
end

last_elem = filtered_arr.last
keys = last_elem.keys

final_tbl = []
wrapper = { items: filtered_arr }
items_to_inject = wrapper[:items]
items_to_inject.each do |it|
  final_tbl << { uid: it["id"], uval: it["val"] }
end

t1 = Time.now
json_ms = (t1 - t0) * 1000.0
puts "json_elapsed_ms: #{'%.3f' % json_ms}"
