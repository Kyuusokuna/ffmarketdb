var selected_item = null;
var selected_dc = null;

var current_listings = [];

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

function update_selection(new_item, new_dc) {
    selected_item = new_item;
    selected_dc = new_dc;

    if(!selected_item || !selected_dc)
        return;
    
    current_listings = [];
    document.getElementById("results_table_rows").replaceChildren();
    document.getElementById("selected_item").innerText = get_item_name(new_item);

    for(const world of selected_dc.Worlds){
        const url = "http://ffmarketdb.kyuusokuna.ovh:3000/items/" + world + "/" + selected_item;
        //const url = "http://localhost:3000/items/" + world + "/" + selected_item;
        const world_name = get_world_name(world)

        $.ajax({
            dataType: "json",
            url: url,
            success: async function(data) {
                if(new_item != selected_item || !selected_dc.Worlds.includes(world))
                    return;

                current_listings.push(...data.listings.map(listing => ({ ...listing, world: world_name})));
                current_listings.sort((a, b) => a.price_per_unit - b.price_per_unit);
                
                table_rows = current_listings
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
            }
        });
    }
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
            selected_dc = dcs[index];
        }

        dc_options.push(option);
    }

    document.getElementById("dc_select").replaceChildren(...dc_options);


    $("#item_select").chosen({
        max_shown_results: 500,
        search_contains: true,
    }).change(function(event, selected) {
        update_selection(selected.selected, selected_dc);
    });

    $("#dc_select").chosen({
        search_contains: true,
    }).change(function(event, selected) {
        set_local_storage("selected_dc", dcs[selected.selected].Name)
        update_selection(selected_item, dcs[selected.selected]);
    });
});