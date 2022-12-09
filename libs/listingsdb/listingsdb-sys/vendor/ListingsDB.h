#pragma once
#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

#define LISTINGSDB_MAX_NUM_LISTINGS_PER_ITEM (100)
#define LISTINGSDB_MAX_NUM_MATERIA_PER_ITEM (5)
#define LISTINGSDB_MAX_RETAINER_NAME_LENGTH (24)

typedef struct {
	uint8_t flags;
	uint8_t city;

	uint16_t dye_id;
	uint16_t materia_ids[LISTINGSDB_MAX_NUM_MATERIA_PER_ITEM];

	uint16_t amount;
	uint32_t price_per_unit;
	
	uint8_t retainer_name[LISTINGSDB_MAX_RETAINER_NAME_LENGTH];
} ListingsDB_Listing;

bool ListingsDB_init(const char *base_path);
void ListingsDB_shutdown();

void ListingsDB_update_listings(uint16_t world_id, uint16_t item_id, uint8_t num_listings, const ListingsDB_Listing *new_listings);
bool ListingsDB_get_listings(uint16_t world_id, uint16_t item_id, uint8_t *num_listings, ListingsDB_Listing *result, uint64_t *last_update_time);

#ifdef __cplusplus
}
#endif