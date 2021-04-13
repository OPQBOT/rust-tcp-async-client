function OnChatMsg(cli, msg)
    --print(
    cli:SendPkg({FromUserName = "1234", ToUserName = "9875"})
    --)

    -- if cli:SendPkg(99) == nil then
    --     print(" cli ", FormatTable(cli))
    --     print("====2111  Test", id)
    -- end
    print("Clientid", cli.clientid)
    print("msg", msg.msg_id, msg.to_user, msg.from_user)

    cli:prints("hello")
    cli.print("world")
    --print(hexdecode("305c02010004553053020100020443cfe7f202032f4dc70204d7b52b6f0204605dec62042e6175706170706d73675f623939626539306265393033323336635f313631363736383039383139305f32363935340204010400030201000400"))
    return 1
end

function OnChatEvent(cli, id)
    print(" cli ", cli)
    print("==== ", id)
    return 1
end

function FormatValue(val)
    if type(val) == "string" then
        return string.format("%q", val)
    end
    return tostring(val)
end

function FormatTable(t, tabcount)
    tabcount = tabcount or 0
    if tabcount > 5 then
        --防止栈溢出
        return "<table too deep>" .. tostring(t)
    end
    local str = ""
    if type(t) == "table" then
        for k, v in pairs(t) do
            local tab = string.rep("\t", tabcount)
            if type(v) == "table" then
                str = str .. tab .. string.format("[%s] = {", FormatValue(k)) .. "\n"
                str = str .. FormatTable(v, tabcount + 1) .. tab .. "}\n"
            else
                str = str .. tab .. string.format("[%s] = %s", FormatValue(k), FormatValue(v)) .. ",\n"
            end
        end
    else
        str = str .. tostring(t) .. "\n"
    end
    return str
end

--print(hexdecode(hexencode("Hello, World!")))
