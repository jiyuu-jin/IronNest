let ips = #{
    filter_pump: "10.0.0.197",
    light_white: "10.0.0.251",
    light_blue: "10.0.0.198",
};

if state["filter.pump.on"] {
    tplink_turn_plug_on(ips["filter_pump"]);
} else {
    tplink_turn_plug_off(ips["filter_pump"]);
}

if state["light.white.on"] {
    tplink_turn_plug_on(ips["light_white"]);
} else {
    tplink_turn_plug_off(ips["light_white"]);
}

if state["light.blue.on"] {
    tplink_turn_plug_on(ips["light_blue"]);
} else {
    tplink_turn_plug_off(ips["light_blue"]);
}

# comment
