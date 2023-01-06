var selected_item = null; // Item ID
var selected_dc = null; // DC Name

var current_listings = [];
var current_request_id = 0;

function get_local_storage(key) {
    try {
        return window.localStorage.getItem(key);
    } catch (e) {
        return null;
    }
}

function set_local_storage(key, value) {
    try {
        window.localStorage.setItem(key, value);
    } catch(e) { }
}

function get_world_name(world_id) {
    const name = worlds.find(world => world.id == world_id).Name;
    return name ? name : "UNKNOWN";
}

function get_item_name(item_id) {
    const name = items.find(({ id }) => id == item_id).Name;
    return name ? name : "UNKNOWN";
}

function create_tr_from_values(...values) {
    let tr = document.createElement("tr");

    for(value of values) {
        let td = document.createElement("td");
        td.innerText = value;
        tr.appendChild(td);
    }

    return tr;
}

function get_time_difference_string(time) {
    if (!time) return "never";

    const seconds = Math.floor(Date.now() / 1000 - time);
    const minutes = Math.floor(seconds / 60);
    const hours = Math.floor(minutes / 60);
    const days = Math.floor(hours / 24);

    if (days > 1) return days + " days ago";
    if (days == 1) return "1 day ago";

    if (hours > 1) return hours + " hours ago";
    if (hours == 1) return "1 hour ago";

    if (minutes > 1) return minutes + " minutes ago";
    if (minutes == 1) return "1 minute ago";

    if (seconds > 5) return seconds + " seconds ago";

    return "just now";
}

function update_shown_data() {
    const enabled_worlds = selected_dc.Worlds.filter((world) => !document.getElementById(world)?.classList?.contains("disabled"));

    for(row of document.getElementById("results_table_rows").children) {
        if(enabled_worlds.map((id) => get_world_name(id)).includes(row.children[5].innerHTML))
            row.classList.remove("hidden_row");
        else
            row.classList.add("hidden_row");
    }
}

function request_data() {
    if(!selected_item || !selected_dc)
        return;

    const request_id = ++current_request_id;
    current_listings = [];
    document.getElementById("results_table_rows").replaceChildren();
    
    for(const world of selected_dc.Worlds) {
        const url = "https://ffmarketdb.kyuusokuna.ovh/items/" + world + "/" + selected_item;
        //const url = "http://localhost:3000/items/" + world + "/" + selected_item;

        const world_name = get_world_name(world);
        document.getElementById(world).innerHTML = world_name + "<br/>Loading...";

        $.ajax({
            dataType: "json",
            url: url,
            success: function(data) {
                if(current_request_id != request_id)
                    return;

                document.getElementById(world).innerHTML = world_name + "<br/>" + get_time_difference_string(data.last_updated);

                current_listings.push(...data.listings.map(listing => ({ ...listing, world: world_name, world_id: world})));
                current_listings.sort((a, b) => a.price_per_unit - b.price_per_unit);

                const table_rows = current_listings
                    .map((listing, index) => 
                        create_tr_from_values(
                            index.toLocaleString("en-US"),
                            listing.flags & 0b00001000 ? "Y" : "",
                            listing.amount.toLocaleString("en-US"),
                            listing.price_per_unit.toLocaleString("en-US"),
                            (listing.amount * listing.price_per_unit).toLocaleString("en-US"),
                            listing.world,
                            listing.retainer_name,
                        )
                    );

                document.getElementById("results_table_rows").replaceChildren(...table_rows);

                update_shown_data();
            }
        });
    }
}

function update_selected_item(new_item) {
    selected_item = new_item;
    document.getElementById("selected_item").innerText = get_item_name(new_item);

    request_data();
}

function update_selected_dc(new_dc) {
    selected_dc = new_dc;
    document.getElementById("server_select").replaceChildren();

    for(const world of selected_dc.Worlds.sort((a, b) => get_world_name(a).localeCompare(get_world_name(b)))) {
        let a = document.createElement("a");
        a.id = world;
        a.innerHTML = get_world_name(world) + "<br/>N/A";
        a.onclick = () => { a.classList.toggle("disabled"); update_shown_data(); };
        document.getElementById("server_select").appendChild(a);
    }

    request_data();
}

$(document).ready(function() {
    let item_options = [ document.createElement("option") ];

    for(const item of items) {
        var option = document.createElement("option");
        option.value = item.id;
        option.text = item.Name;

        item_options.push(option);
    }

    document.getElementById("item_select").replaceChildren(...item_options);

    let dc_options = [ document.createElement("option") ];
    const stored_dc = get_local_storage("selected_dc");

    for(const [index, dc] of dcs.entries()) {
        var option = document.createElement("option");
        option.value = index;
        option.text = dc.Name;

        if(stored_dc && stored_dc == dc.Name) {
            option.selected = true;
            update_selected_dc(dc);
        }

        dc_options.push(option);
    }

    document.getElementById("dc_select").replaceChildren(...dc_options);

    $("#item_select").chosen({
        max_shown_results: 500,
        search_contains: true,
    }).change(function(event, selected) {
        update_selected_item(selected.selected);
    });

    $("#dc_select").chosen({
        search_contains: true,
    }).change(function(event, selected) {
        set_local_storage("selected_dc", dcs[selected.selected].Name)
        update_selected_dc(dcs[selected.selected]);
    });
});