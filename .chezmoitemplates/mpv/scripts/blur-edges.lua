require "mp.msg"
local options = require 'mp.options'

local opts = {
  blur_radius = 10,
  blur_power = 10,
  minimum_black_bar_size = 3,
  mode = "all",
  active = true,
  reapply_delay = 0.5,
  watch_later_fix = false,
  only_fullscreen = true,
  prepend_subs = false
}
options.read_options(opts)

local active = opts.active

local label_prefix = mp.get_script_name()
local labels = {
  blur = string.format("%s-blur", label_prefix),
  sub = string.format("%s-sub", label_prefix)
}

function is_filter_present(label)
  local filters = mp.get_property_native("vf")
  for index, filter in pairs(filters) do
    if filter["label"] == label then
      return true
    end
  end
  return false
end

function remove_filter(label)
  if is_filter_present(label) then
    mp.command(string.format('no-osd vf remove @%s', label))
    return true
  end
  return false
end

function cleanup()
  -- Remove all existing filters.
  for key, value in pairs(labels) do
    remove_filter(value)
  end
end

function set_blur()
  unset_blur()

  if not mp.get_property("video-out-params") then
    return
  end
  if opts.only_fullscreen and not mp.get_property_bool("fullscreen") then
    return
  end
  local video_aspect = mp.get_property_number("video-out-params/aspect")
  local ww, wh = mp.get_osd_size()

  if math.abs(ww / wh - video_aspect) < 0.05 then
    return
  end
  if opts.mode == "horizontal" and ww / wh < video_aspect then
    return
  end
  if opts.mode == "vertical" and ww / wh > video_aspect then
    return
  end

  local par = mp.get_property_number("video-out-params/par")
  local height = mp.get_property_number("video-out-params/h")
  local width = mp.get_property_number("video-out-params/w")

  local split = "split=3 [a] [v] [b]"
  local crop_format = "crop=%s:%s:%s:%s"
  local scale_format = "scale=width=%s:height=%s:flags=neighbor"

  local stack_direction, cropped_scaled_1, cropped_scaled_2, blur_size

  if ww / wh > video_aspect then
    blur_size = math.floor((ww / wh) * height / par - width)
    local nudge = blur_size % 2
    blur_size = blur_size / 2

    local height_with_maximized_width = height / width * ww
    local visible_height = math.floor(height * par * wh / height_with_maximized_width)
    local visible_width = math.floor(blur_size * wh / height_with_maximized_width)

    local cropped_1 = string.format(crop_format, visible_width, visible_height, "0", (height - visible_height) / 2)
    local scaled_1 = string.format(scale_format, blur_size + nudge, height)
    cropped_scaled_1 = cropped_1 .. "," .. scaled_1

    local cropped_2 = string.format(crop_format, visible_width, visible_height, width - visible_width,
      (height - visible_height) / 2)
    local scaled_2 = string.format(scale_format, blur_size, height)
    cropped_scaled_2 = cropped_2 .. "," .. scaled_2
    stack_direction = "h"
  else
    blur_size = math.floor((wh / ww) * width * par - height)
    local nudge = blur_size % 2
    blur_size = blur_size / 2

    local width_with_maximized_height = width / height * wh
    local visible_width = math.floor(width * ww / width_with_maximized_height)
    local visible_height = math.floor(blur_size * ww / width_with_maximized_height)

    local cropped_1 = string.format(crop_format, visible_width, visible_height, (width - visible_width) / 2, "0")
    local scaled_1 = string.format(scale_format, width, blur_size + nudge)
    cropped_scaled_1 = cropped_1 .. "," .. scaled_1

    local cropped_2 = string.format(crop_format, visible_width, visible_height, (width - visible_width) / 2,
      height - visible_height)
    local scaled_2 = string.format(scale_format, width, blur_size)
    cropped_scaled_2 = cropped_2 .. "," .. scaled_2
    stack_direction = "v"
  end

  if blur_size < math.max(1, opts.minimum_black_bar_size) then
    return
  end
  local lr = math.min(opts.blur_radius, math.floor(blur_size / 2) - 1)
  local cr = math.min(opts.blur_radius, math.floor(blur_size / 4) - 1)
  local blur = string.format("boxblur=lr=%i:lp=%i:cr=%i:cp=%i", lr, opts.blur_power, cr, opts.blur_power)
  -- local blur = string.format("gblur=sigma=30:steps=3")
  -- local blur = string.format("boxblur=lr=20:lp=10")

  zone_1 = string.format("[a] %s,%s [a_fin]", cropped_scaled_1, blur)
  zone_2 = string.format("[b] %s,%s [b_fin]", cropped_scaled_2, blur)

  local par_fix = "setsar=ratio=" .. tostring(par) .. ":max=10000"

  stack = string.format("[a_fin] [v] [b_fin] %sstack=3,%s", stack_direction, par_fix)
  filter = string.format("%s;%s;%s;%s", split, zone_1, zone_2, stack)
  if opts.prepend_subs then
    mp.command(string.format("no-osd vf add '@%s:sub'", labels.sub))
  end
  mp.command(string.format("no-osd vf add '@%s:lavfi=\"%s\":o=\"threads=8,thread_type=slice\"'", labels.blur, filter))
end

function unset_blur()
  if opts.prepend_subs then
    remove_filter(labels.sub)
  end
  remove_filter(labels.blur)
end

local reapplication_timer = mp.add_timeout(opts.reapply_delay, set_blur)
reapplication_timer:kill()

function reset_blur(k, v)
  unset_blur()
  reapplication_timer:kill()
  reapplication_timer:resume()
end

local curr_crop = nil

function on_filter_change()
  local filters = mp.get_property_native("vf")
  local new_crop = nil
  for index, filter in pairs(filters) do
    if filter["name"] == "lavfi-crop" or filter["name"] == "crop" then
      new_crop = filter
      break
    end
  end

  if curr_crop == nil then
    if new_crop ~= nil then
      curr_crop = new_crop
      reset_blur()
    end
  else
    if new_crop == nil then
      curr_crop = nil
      reset_blur()
    elseif new_crop["params"]["w"] ~= curr_crop["params"]["w"] or new_crop["params"]["h"] ~= curr_crop["params"]["h"] or new_crop["enabled"] ~= curr_crop["enabled"] then
      curr_crop = new_crop
      reset_blur()
    end
  end
end

function toggle()
  if active then
    active = false
    unset_blur()
    mp.unobserve_property(reset_blur)
    mp.unobserve_property(on_filter_change)
    mp.msg.info("unset")
  else
    active = true
    set_blur()
    local properties = { "osd-width", "osd-height", "path", "fullscreen" }
    for _, p in ipairs(properties) do
      mp.observe_property(p, "native", reset_blur)
    end
    mp.observe_property("vf", "native", on_filter_change)
  end
end

if active then
  active = false
  toggle()
end

mp.add_key_binding(nil, "toggle-blur", toggle)
mp.add_key_binding(nil, "set-blur", set_blur)
mp.add_key_binding(nil, "unset-blur", unset_blur)
