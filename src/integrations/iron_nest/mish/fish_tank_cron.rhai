let ips = #{
    filter_pump: "10.0.0.197",
    light_white: "10.0.0.251",
    light_blue: "10.0.0.198",
};

if is_now_between("America/New_York", "07:00:00", "21:00:00") {
    update_mish_state("chris.fish_tank", "$['filter.pump.on']", true);
} else {
    update_mish_state("chris.fish_tank", "$['filter.pump.on']", false);
}

if is_now_between("America/New_York", "06:00:00", "20:00:00") {
    update_mish_state("chris.fish_tank", "$['light.white.on']", true);
} else {
    update_mish_state("chris.fish_tank", "$['light.white.on']", false);
}

if is_now_between("America/New_York", "05:00:00", "22:00:00") {
    update_mish_state("chris.fish_tank", "$['light.blue.on']", true);
} else {
    update_mish_state("chris.fish_tank", "$['light.blue.on']", false);
}
