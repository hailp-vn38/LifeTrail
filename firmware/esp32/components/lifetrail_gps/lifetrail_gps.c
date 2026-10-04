#include "lifetrail_gps.h"

#include <string.h>

#include "lifetrail_nmea.h"

#define LT_GPS_EPOCH_CACHE_MS UINT64_C(2000)
#define LT_GPS_CACHE_CAPACITY 4U

typedef struct {
  bool present;
  uint32_t second_of_day;
  uint64_t received_at_ms;
  lt_gps_record_t record;
} rmc_epoch_t;

typedef struct {
  bool present;
  uint32_t second_of_day;
  uint64_t received_at_ms;
  lt_nmea_gga_t gga;
} gga_epoch_t;

typedef struct {
  rmc_epoch_t rmc[LT_GPS_CACHE_CAPACITY];
  gga_epoch_t gga[LT_GPS_CACHE_CAPACITY];
  lt_gps_record_sink_t record_sink;
  void *record_sink_context;
  lt_gps_diagnostics_t diagnostics;
  int64_t last_emitted_ts_ms;
} collector_impl_t;

_Static_assert(sizeof(collector_impl_t) <= sizeof(lt_gps_collector_t),
               "collector state is too small");

static collector_impl_t *collector_impl(lt_gps_collector_t *collector) {
  return (collector_impl_t *)&collector->state;
}

static const collector_impl_t *collector_impl_const(
    const lt_gps_collector_t *collector) {
  return (const collector_impl_t *)&collector->state;
}

static bool is_expired(uint64_t now_ms, uint64_t then_ms) {
  return now_ms >= then_ms && now_ms - then_ms > LT_GPS_EPOCH_CACHE_MS;
}

static void expire_epochs(collector_impl_t *impl, uint64_t now_ms) {
  size_t index;

  for (index = 0U; index < LT_GPS_CACHE_CAPACITY; index++) {
    if (impl->rmc[index].present &&
        is_expired(now_ms, impl->rmc[index].received_at_ms)) {
      impl->rmc[index].present = false;
      impl->diagnostics.navigation_epoch_unmatched++;
    }
    if (impl->gga[index].present &&
        is_expired(now_ms, impl->gga[index].received_at_ms)) {
      impl->gga[index].present = false;
      impl->diagnostics.navigation_epoch_unmatched++;
    }
  }
}

static size_t vacant_rmc_slot(collector_impl_t *impl) {
  size_t index;
  for (index = 0U; index < LT_GPS_CACHE_CAPACITY; index++) {
    if (!impl->rmc[index].present) {
      return index;
    }
  }
  impl->diagnostics.navigation_epoch_unmatched++;
  return 0U;
}

static size_t vacant_gga_slot(collector_impl_t *impl) {
  size_t index;
  for (index = 0U; index < LT_GPS_CACHE_CAPACITY; index++) {
    if (!impl->gga[index].present) {
      return index;
    }
  }
  impl->diagnostics.navigation_epoch_unmatched++;
  return 0U;
}

static void merge_epochs(collector_impl_t *impl) {
  size_t rmc_index;
  size_t gga_index;

  for (rmc_index = 0U; rmc_index < LT_GPS_CACHE_CAPACITY; rmc_index++) {
    if (!impl->rmc[rmc_index].present) {
      continue;
    }
    for (gga_index = 0U; gga_index < LT_GPS_CACHE_CAPACITY; gga_index++) {
      lt_gps_record_t record;
      const lt_nmea_gga_t *gga;
      if (!impl->gga[gga_index].present ||
          impl->rmc[rmc_index].second_of_day !=
              impl->gga[gga_index].second_of_day) {
        continue;
      }
      record = impl->rmc[rmc_index].record;
      gga = &impl->gga[gga_index].gga;
      record.alt_m = gga->alt_m;
      record.hdop = gga->hdop;
      record.fix_quality = gga->fix_quality;
      record.satellites = gga->satellites;
      record.has_alt_m = gga->has_alt_m;
      record.has_hdop = gga->has_hdop;
      impl->rmc[rmc_index].present = false;
      impl->gga[gga_index].present = false;
      if (record.ts_ms <= impl->last_emitted_ts_ms) {
        impl->diagnostics.duplicate_epoch++;
        break;
      }
      impl->last_emitted_ts_ms = record.ts_ms;
      if (impl->record_sink != NULL) {
        impl->record_sink(impl->record_sink_context, &record);
      }
      break;
    }
  }
}

static void cache_rmc(collector_impl_t *impl, const lt_nmea_rmc_t *rmc,
                      uint64_t arrival_ms) {
  size_t slot = vacant_rmc_slot(impl);
  impl->rmc[slot] =
      (rmc_epoch_t){true, rmc->second_of_day, arrival_ms, rmc->record};
}

static void cache_gga(collector_impl_t *impl, const lt_nmea_gga_t *gga,
                      uint64_t arrival_ms) {
  size_t slot = vacant_gga_slot(impl);
  impl->gga[slot] = (gga_epoch_t){true, gga->second_of_day, arrival_ms, *gga};
}

void lt_gps_collector_init(lt_gps_collector_t *collector,
                           lt_gps_record_sink_t record_sink,
                           void *record_sink_context) {
  collector_impl_t *impl = collector_impl(collector);
  memset(impl, 0, sizeof(*impl));
  impl->record_sink = record_sink;
  impl->record_sink_context = record_sink_context;
}

bool lt_gps_collector_ingest_nmea(lt_gps_collector_t *collector,
                                  const char *sentence,
                                  uint64_t arrival_monotonic_ms) {
  collector_impl_t *impl = collector_impl(collector);
  lt_nmea_rmc_t rmc = {0};
  lt_nmea_gga_t gga = {0};
  lt_nmea_parse_result_t result;

  expire_epochs(impl, arrival_monotonic_ms);
  result = lt_nmea_parse(sentence, &rmc, &gga);
  if (result == LT_NMEA_INVALID_CHECKSUM) {
    impl->diagnostics.invalid_checksum++;
    return false;
  }
  if (result == LT_NMEA_INVALID_RECORD) {
    impl->diagnostics.invalid_rmc++;
    return false;
  }
  if (result == LT_NMEA_RMC) {
    cache_rmc(impl, &rmc, arrival_monotonic_ms);
  } else if (result == LT_NMEA_GGA) {
    cache_gga(impl, &gga, arrival_monotonic_ms);
  } else {
    return true;
  }
  merge_epochs(impl);
  return true;
}

void lt_gps_collector_expire(lt_gps_collector_t *collector,
                             uint64_t now_monotonic_ms) {
  expire_epochs(collector_impl(collector), now_monotonic_ms);
}

lt_gps_diagnostics_t lt_gps_collector_diagnostics(
    const lt_gps_collector_t *collector) {
  return collector_impl_const(collector)->diagnostics;
}
