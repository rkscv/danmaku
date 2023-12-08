-- https://github.com/po5/mpv_irc/blob/master/scrollingsubs.lua

local utils = require('mp.utils')

local duration = 9
local interval = .001
local danmaku = nil
local ov = mp.create_osd_overlay('ass-events')

local function render()
    local w, h = mp.get_osd_size()
    if w == 0 or h == 0 then
        return
    end
    local pos, error = mp.get_property_number('time-pos')
    if error then
        mp.msg.error(error)
        return
    end
    local size, error = mp.get_property_number('osd-font-size')
    if error then
        mp.msg.error(error)
        return
    end
    local speed, error = mp.get_property_number('speed')
    if error then
        mp.msg.error(error)
        return
    end
    -- https://mpv.io/manual/stable/#options-sub-font-size
    size = size / 720 * h / 2
    local spacing = size / 10
    local rows = {}
    for i = 1, math.max(h / (size + spacing), 1) do
        rows[i] = -math.huge
    end

    local data = ''
    for _, danmaku in ipairs(danmaku) do
        if danmaku.duration > pos + duration then
            break
        end

        if not danmaku.x then
            danmaku.x = w - (pos - danmaku.duration) * (w / duration)
        end
        if danmaku.x + #danmaku.message * size < 0 then
            goto continue
        end
        if not danmaku.y then
            for index, row in ipairs(rows) do
                if row < danmaku.x then
                    danmaku.y = index
                    break
                end
            end
            if not danmaku.y then
                for index, row in ipairs(rows) do
                    if not danmaku.y or row < rows[danmaku.y] then
                        danmaku.y = index
                    end
                end
            end
        end

        if #data > 0 then
            data = data .. '\n'
        end
        data = data ..
            string.format('{\\pos(%f,%f)\\c&H%x%x%x&\\alpha&H30\\fscx50\\fscy50\\bord1.5\\b1\\q2}%s', danmaku.x,
                (danmaku.y - 1) * (size + spacing),
                danmaku.b,
                danmaku.g,
                danmaku.r,
                danmaku.message:gsub("\n", "\\N"))
        danmaku.x = danmaku.x - w / duration * speed * interval
        rows[danmaku.y] = math.max(rows[danmaku.y] or -math.huge, danmaku.x + #danmaku.message * size)
        ::continue::
    end
    ov.data = data
    ov.res_x = w
    ov.res_y = h
    ov:update()
end

local function reset()
    for _, danmaku in ipairs(danmaku) do
        danmaku.x = nil
        danmaku.y = nil
    end
    render()
end

local function loaded(n)
    local message = string.format('Loaded %d danmaku comment', n)
    if n > 1 then
        message = message .. 's'
    end
    mp.osd_message(message)
end

local timer = mp.add_periodic_timer(interval, render, true)
local enabled = true

mp.register_event('file-loaded', function(event)
    timer:kill()
    danmaku = nil
    if enabled then
        ov.format = 'none'
        ov:update()
        ov.format = 'ass-events'
    end
    if event.error then
        mp.msg.error(event.error)
    end
end)
mp.register_script_message('toggle-danmaku', function()
    enabled = not enabled
    if enabled then
        if danmaku then
            reset()
            local pause, error = mp.get_property_bool('pause')
            if error then
                mp.msg.error(error)
                return
            end
            if pause then
                render()
            else
                timer:resume()
            end
            loaded(#danmaku)
        else
            mp.osd_message('Danmaku: on')
        end
    else
        timer:kill()
        ov.format = 'none'
        ov:update()
        ov.format = 'ass-events'
        mp.osd_message('Danmaku: off')
    end
end)
mp.register_script_message('emit-danmaku', function(json)
    local json, error = utils.parse_json(json)
    if error then
        mp.msg.error(error)
        return
    end
    danmaku = json
    if enabled then
        local pause, error = mp.get_property_bool('pause')
        if error then
            mp.msg.error(error)
            return
        end
        if pause then
            render()
        else
            timer:resume()
        end
        loaded(#danmaku)
    end
end)
mp.observe_property('pause', 'bool', function(_, value)
    if enabled and danmaku then
        if value then
            timer:kill()
        else
            timer:resume()
        end
    end
end)
mp.observe_property('seeking', nil, function()
    if enabled and danmaku then
        reset()
    end
end)
