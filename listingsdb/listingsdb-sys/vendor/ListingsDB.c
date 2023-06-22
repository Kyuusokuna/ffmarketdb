#include "ListingsDB.h"

#include <stdlib.h>
#include <string.h>
#include <time.h>
#define CUTE_FILES_IMPLEMENTATION
#include "cute_files.h"
#define SHARED_MUTEX_IMPLEMENTATION
#include "shared_mutex.h"

#define HIGHEST_WORLD_ID (UINT16_MAX)
#define HIGHEST_ITEM_ID (UINT16_MAX)

#define MAX_NUM_WORLDS (256)
#define MAX_NUM_ITEMS (16384)

#define ListingsDB_min(a, b) (a < b ? a : b)
#define ListingsDB_max(a, b) (a > b ? a : b)

typedef struct {
	uint32_t highest_used_index;
	uint8_t  game_id_to_index[HIGHEST_WORLD_ID + 1];
	uint16_t index_to_game_id[MAX_NUM_WORLDS];
} World_ID_Mappings;

typedef struct {
	uint32_t highest_used_index;
	uint16_t game_id_to_index[HIGHEST_ITEM_ID + 1];
	uint16_t index_to_game_id[MAX_NUM_ITEMS];
} Item_ID_Mappings;

typedef struct {
	uint64_t last_update_time;

	uint8_t num_active_listings;
	ListingsDB_Listing listings[LISTINGSDB_MAX_NUM_LISTINGS_PER_ITEM];
} ListingsDB_Item_Listings;

typedef struct {
	ListingsDB_Item_Listings items[MAX_NUM_ITEMS];
} World_Data;

static bool initialized;
static Shared_Mutex initialization_mutex = SHARED_MUTEX_INITIALIZER;

static char data_path[1024];

static World_ID_Mappings world_id_mappings;
static Item_ID_Mappings *item_id_mappings;
static Shared_Mutex id_mappings_mutex;

static World_Data *world_data[MAX_NUM_WORLDS];
static Shared_Mutex world_data_mutex[MAX_NUM_WORLDS];

static void *map_file_writeable(const char *path, uint32_t size);
static void unmap_file(void *start, size_t length);
static void ListingsDB_shutdown_internal();

bool ListingsDB_init(const char *base_path) {
	Shared_Mutex_lock_writing(&initialization_mutex);

	if (initialized) {
		Shared_Mutex_unlock_writing(&initialization_mutex);
		return true;
	}

	size_t base_path_length = strlen(base_path);

	if (!base_path || base_path_length > 1023) {
		Shared_Mutex_unlock_writing(&initialization_mutex);
		return false;
	}

	memcpy(data_path, base_path, base_path_length + 1);

	char filepath_buf[2048];
	snprintf(filepath_buf, sizeof(filepath_buf), "%s%s", data_path, "/item_id_mappings");

	item_id_mappings = map_file_writeable(filepath_buf, sizeof(*item_id_mappings));
	if (!item_id_mappings) {
		Shared_Mutex_unlock_writing(&initialization_mutex);
		return false;
	}

	cf_dir_t directory;
	if (!cf_dir_open(&directory, base_path)) {
		ListingsDB_shutdown_internal();
		Shared_Mutex_unlock_writing(&initialization_mutex);
		return false;
	}

	const char *digits = "0123456789";
	for (; directory.has_next; cf_dir_next(&directory)) {
		cf_file_t file;
		cf_read_file(&directory, &file);
		if (file.is_dir)
			continue;

		size_t filename_length = strlen(file.name);
		if (!filename_length || filename_length != strspn(file.name, digits))
			continue;

		int world_id = atoi(file.name);
		if (!world_id || world_id > HIGHEST_WORLD_ID)
			continue;

		uint8_t world_index = world_id_mappings.game_id_to_index[world_id];
		if (world_index || world_id_mappings.highest_used_index + 1 > MAX_NUM_WORLDS) {
			ListingsDB_shutdown_internal(); 
			Shared_Mutex_unlock_writing(&initialization_mutex);
			return false;
		}

		world_id_mappings.highest_used_index++;
		world_id_mappings.game_id_to_index[world_id] = world_id_mappings.highest_used_index;
		world_id_mappings.index_to_game_id[world_id_mappings.highest_used_index] = world_id;

		world_index = world_id_mappings.game_id_to_index[world_id];

		if (world_data[world_index]) {
			ListingsDB_shutdown_internal();
			Shared_Mutex_unlock_writing(&initialization_mutex);
			return false;
		}

		world_data[world_index] = map_file_writeable(file.path, sizeof(*world_data[world_index]));
		if (!world_data[world_index]) {
			ListingsDB_shutdown_internal();
			Shared_Mutex_unlock_writing(&initialization_mutex);
			return false;
		}
	}

	cf_dir_close(&directory);

	initialized = true;

	Shared_Mutex_unlock_writing(&initialization_mutex);
	return true;
}

static void ListingsDB_shutdown_internal() {
	initialized = false;

	if (item_id_mappings) {
		unmap_file(item_id_mappings, sizeof(*item_id_mappings));
		item_id_mappings = 0;
	}

	for (int i = 0; i < MAX_NUM_WORLDS; i++) {
		if (!world_data[i])
			continue;

		unmap_file(world_data[i], sizeof(*world_data[i]));
		world_data[i] = 0;
	}
}

void ListingsDB_shutdown() {
	Shared_Mutex_lock_writing(&initialization_mutex);
	ListingsDB_shutdown_internal();
	Shared_Mutex_unlock_writing(&initialization_mutex);
}

void ListingsDB_update_listings(uint16_t world_id, uint16_t item_id, uint8_t num_listings, const ListingsDB_Listing *new_listings) {
	Shared_Mutex_lock_reading(&initialization_mutex);
	if (!initialized || !num_listings || !new_listings) {
		Shared_Mutex_unlock_reading(&initialization_mutex);
		return;
	}

	Shared_Mutex_lock_reading(&id_mappings_mutex);
	uint8_t world_index = world_id_mappings.game_id_to_index[world_id];
	uint16_t item_index = item_id_mappings->game_id_to_index[item_id];
	Shared_Mutex_unlock_reading(&id_mappings_mutex);

	if (!world_index) {
		Shared_Mutex_lock_writing(&id_mappings_mutex);

		if (!world_id_mappings.game_id_to_index[world_id]) {
			if (world_id_mappings.highest_used_index + 1 > MAX_NUM_WORLDS) {
				Shared_Mutex_unlock_writing(&id_mappings_mutex);
				Shared_Mutex_unlock_reading(&initialization_mutex);
				return;
			}

			world_id_mappings.highest_used_index++;
			world_id_mappings.game_id_to_index[world_id] = world_id_mappings.highest_used_index;
			world_id_mappings.index_to_game_id[world_id_mappings.highest_used_index] = world_id;
		}

		world_index = world_id_mappings.game_id_to_index[world_id];

		Shared_Mutex_unlock_writing(&id_mappings_mutex);
	}

	if (!item_index) {
		Shared_Mutex_lock_writing(&id_mappings_mutex);

		if (!item_id_mappings->game_id_to_index[item_id]) {
			if (item_id_mappings->highest_used_index + 1 > MAX_NUM_ITEMS) {
				Shared_Mutex_unlock_writing(&id_mappings_mutex);
				Shared_Mutex_unlock_reading(&initialization_mutex);
				return;
			}

			item_id_mappings->highest_used_index++;
			item_id_mappings->game_id_to_index[item_id] = item_id_mappings->highest_used_index;
			item_id_mappings->index_to_game_id[item_id_mappings->highest_used_index] = item_id;
		}

		item_index = item_id_mappings->game_id_to_index[item_id];

		Shared_Mutex_unlock_writing(&id_mappings_mutex);
	}

	Shared_Mutex_lock_reading(&world_data_mutex[world_index]);
	World_Data *data = world_data[world_index];
	Shared_Mutex_unlock_reading(&world_data_mutex[world_index]);

	if (!data) {
		Shared_Mutex_lock_writing(&world_data_mutex[world_index]);

		if (!world_data[world_index]) {
			char filepath_buf[2048];
			snprintf(filepath_buf, sizeof(filepath_buf), "%s/%hu", data_path, world_id);

			world_data[world_index] = map_file_writeable(filepath_buf, sizeof(*world_data[world_index]));
		}

		data = world_data[world_index];

		Shared_Mutex_unlock_writing(&world_data_mutex[world_index]);
	}

	if (!data) {
		Shared_Mutex_unlock_reading(&initialization_mutex);
		return;
	}

	Shared_Mutex_lock_writing(&world_data_mutex[world_index]);

	data->items[item_index].last_update_time = time(0);
	data->items[item_index].num_active_listings = ListingsDB_min(num_listings, 100);

	for (int i = 0; i < ListingsDB_min(num_listings, 100); i++)
		data->items[item_index].listings[i] = new_listings[i];

	Shared_Mutex_unlock_writing(&world_data_mutex[world_index]);
	Shared_Mutex_unlock_reading(&initialization_mutex);
}

bool ListingsDB_get_listings(uint16_t world_id, uint16_t item_id, uint8_t *num_listings, ListingsDB_Listing *result, uint64_t *last_update_time) {
	Shared_Mutex_lock_reading(&initialization_mutex);
	if (!initialized)
		return false;

	Shared_Mutex_lock_reading(&id_mappings_mutex);
	uint8_t world_index = world_id_mappings.game_id_to_index[world_id];
	uint16_t item_index = item_id_mappings->game_id_to_index[item_id];
	Shared_Mutex_unlock_reading(&id_mappings_mutex);

	if (!world_index || !item_index) {
		Shared_Mutex_unlock_reading(&initialization_mutex);
		return false;
	}

	Shared_Mutex_lock_reading(&world_data_mutex[world_index]);
	ListingsDB_Item_Listings listings = world_data[world_index]->items[item_index];

	*num_listings = listings.num_active_listings;

	for (int i = 0; i < listings.num_active_listings; i++)
		result[i] = listings.listings[i];

	if (last_update_time)
		*last_update_time = listings.last_update_time;

	Shared_Mutex_unlock_reading(&world_data_mutex[world_index]);
	Shared_Mutex_unlock_reading(&initialization_mutex);
	return true;
}

#if _WIN32
	#include <Windows.h>
	
	static void *map_file_writeable(const char *path, uint32_t size) {
		HANDLE file = CreateFileA(path, GENERIC_READ | GENERIC_WRITE, FILE_SHARE_READ, 0, OPEN_ALWAYS, FILE_ATTRIBUTE_NORMAL, 0);
		if (file == INVALID_HANDLE_VALUE)
			return 0;

		HANDLE mapping = CreateFileMappingA(file, 0, PAGE_READWRITE, 0, size, 0);
		CloseHandle(file);
		if (!mapping)
			return 0;

		void *result = MapViewOfFile(mapping, FILE_MAP_READ | FILE_MAP_WRITE, 0, 0, 0);
		CloseHandle(mapping);

		return result;
	}

	static void unmap_file(void *start, size_t length) {
		UnmapViewOfFile(start);
	}
#elif __linux__
	#include <fcntl.h>
	#include <sys/mman.h>
	
	void *map_file_writeable(const char *path, uint32_t size) {
		int file = open(path, O_RDWR | O_CREAT, 0644);
		if (file == -1)
			return 0;

		if (ftruncate(file, size))
			return 0;

		void *result = mmap(0, size, PROT_READ | PROT_WRITE, MAP_SHARED, file, 0);
		close(file);
		if (result == MAP_FAILED)
			return 0;

		return result;
	}

	void unmap_file(void *start, size_t length) {
		munmap(start, length);
	}
#else
	#error Unsupported OS
#endif