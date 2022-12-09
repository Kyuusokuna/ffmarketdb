#pragma once

#ifdef __cplusplus
extern "C" {
#endif

#if _WIN32
	#include <Windows.h>
	#define SHARED_MUTEX_INITIALIZER { SRWLOCK_INIT }

	typedef struct {
		SRWLOCK os_handle;
	} Shared_Mutex;
#elif __linux__
	#include <pthread.h>
	#define SHARED_MUTEX_INITIALIZER { PTHREAD_RWLOCK_INITIALIZER }

	typedef struct {
		pthread_rwlock_t os_handle;
	} Shared_Mutex;
#else
	#error Unsupported OS
#endif

void Shared_Mutex_init(Shared_Mutex *shared_mutex);

void Shared_Mutex_lock_reading(Shared_Mutex *shared_mutex);
void Shared_Mutex_lock_writing(Shared_Mutex *shared_mutex);

void Shared_Mutex_unlock_reading(Shared_Mutex *shared_mutex);
void Shared_Mutex_unlock_writing(Shared_Mutex *shared_mutex);

#ifdef SHARED_MUTEX_IMPLEMENTATION
	#ifndef SHARED_MUREX_IMPLEMENTATION_ONCE
		#define SHARED_MUREX_IMPLEMENTATION_ONCE
		
		#if _WIN32
			void Shared_Mutex_init(Shared_Mutex *shared_mutex) {
				InitializeSRWLock(&shared_mutex->os_handle);
			}

			void Shared_Mutex_lock_reading(Shared_Mutex *shared_mutex) {
				AcquireSRWLockShared(&shared_mutex->os_handle);
			}

			void Shared_Mutex_lock_writing(Shared_Mutex *shared_mutex) {
				AcquireSRWLockExclusive(&shared_mutex->os_handle);
			}

			void Shared_Mutex_unlock_reading(Shared_Mutex *shared_mutex) {
				ReleaseSRWLockShared(&shared_mutex->os_handle);
			}

			void Shared_Mutex_unlock_writing(Shared_Mutex *shared_mutex) {
				ReleaseSRWLockExclusive(&shared_mutex->os_handle);
			}
		#elif __linux__
			void Shared_Mutex_init(Shared_Mutex *shared_mutex) {
				pthread_rwlock_init(&shared_mutex->os_handle, 0);
			}

			void Shared_Mutex_lock_reading(Shared_Mutex *shared_mutex) {
				pthread_rwlock_rdlock(&shared_mutex->os_handle);
			}

			void Shared_Mutex_lock_writing(Shared_Mutex *shared_mutex) {
				pthread_rwlock_wrlock(&shared_mutex->os_handle);
			}

			void Shared_Mutex_unlock_reading(Shared_Mutex *shared_mutex) {
				pthread_rwlock_unlock(&shared_mutex->os_handle);
			}

			void Shared_Mutex_unlock_writing(Shared_Mutex *shared_mutex) {
				pthread_rwlock_unlock(&shared_mutex->os_handle);
			}
		#else
			#error Unsupported OS
		#endif
	#endif
#endif

#ifdef __cplusplus
}
#endif
