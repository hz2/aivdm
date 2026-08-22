/* Minimal C smoke test for the aivdm C FFI: links against libaivdm_ffi and
 * exercises the header end to end against a real captured AIS sentence
 * (the same fixture verified in aivdm's own real_world_corpus tests). */

#include <stdio.h>
#include <string.h>

#include "aivdm.h"

static int failures = 0;

#define CHECK(cond)                                                          \
  do {                                                                       \
    if (!(cond)) {                                                           \
      fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #cond);        \
      failures++;                                                            \
    }                                                                        \
  } while (0)

int main(void) {
  printf("aivdm version: %s\n", aivdm_version());

  /* real captured sentence, message type 1, mmsi 366053209, San Francisco Bay */
  const char *line = "!AIVDM,1,1,,B,15M67FC000G?ufbE`FepT@3n00Sa,0*5C";

  AivdmError err = AIVDM_ERROR_OK;
  AivdmMessage *msg = aivdm_decode_line(line, &err);
  CHECK(msg != NULL);
  CHECK(err == AIVDM_ERROR_OK);
  CHECK(aivdm_message_type(msg) == 1);
  CHECK(aivdm_message_mmsi(msg) == 366053209u);

  double lat = 0.0, lon = 0.0;
  CHECK(aivdm_message_position(msg, &lat, &lon));
  CHECK(lat > 37.80 && lat < 37.81);
  CHECK(lon > -122.35 && lon < -122.34);

  double sog = -1.0;
  CHECK(aivdm_message_sog_knots(msg, &sog));
  CHECK(sog == 0.0);

  uint8_t nav_status = 255;
  CHECK(aivdm_message_navigation_status(msg, &nav_status));
  CHECK(nav_status == 3); /* restricted maneuverability */

  aivdm_message_free(msg);

  /* NULL handling: decode errors must not crash, and accessors on NULL must
   * return their documented defaults rather than dereferencing anything. */
  AivdmError bad_err = AIVDM_ERROR_OK;
  AivdmMessage *bad = aivdm_decode_line("not a valid sentence", &bad_err);
  CHECK(bad == NULL);
  CHECK(bad_err == AIVDM_ERROR_NMEA);

  CHECK(aivdm_decode_line(NULL, &bad_err) == NULL);
  CHECK(bad_err == AIVDM_ERROR_NULL_INPUT);

  CHECK(aivdm_message_type(NULL) == 0);
  CHECK(aivdm_message_mmsi(NULL) == 0);
  double unused;
  CHECK(!aivdm_message_position(NULL, &unused, &unused));

  aivdm_message_free(NULL); /* must be a safe no-op */

  if (failures == 0) {
    printf("all checks passed\n");
    return 0;
  }
  fprintf(stderr, "%d check(s) failed\n", failures);
  return 1;
}
