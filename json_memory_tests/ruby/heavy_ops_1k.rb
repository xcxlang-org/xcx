require 'json'
require 'time'

payload_tbl = []
1000.times do |i|
  payload_tbl << { id: i, name: "User_#{i}", active: true }
end

t0 = Time.now
tbl_json = JSON.parse(JSON.generate(payload_tbl))
root_json = { items: [], meta: { total: 0, processed: false } }
tbl_json.each_with_index do |item, i|
  name = item["name"]
  profile = { id: i, username: name, ratings: [10, 20, 30], flags: { valid: true } }
  root_json[:items] << profile
end
root_json[:meta][:total] = tbl_json.length
root_json[:meta][:processed] = true

serialized_str = JSON.generate(root_json)
parsed = JSON.parse(serialized_str)
meta = parsed["meta"]
total_val = meta["total"]
sample_idx = total_val / 2
sample = parsed["items"][sample_idx]
name_check = sample["username"]

imported = []
wrapper = parsed["items"]
wrapper.each do |it|
  imported << { uid: it["id"], uname: it["username"] }
end

t1 = Time.now
json_ms = (t1 - t0) * 1000.0
puts "json_elapsed_ms: #{'%.3f' % json_ms}"
