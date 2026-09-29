package chef;

import org.junit.jupiter.api.Test;
import java.time.Instant;
import static org.junit.jupiter.api.Assertions.*;

class HttpObservationTest {
    private static final Instant FIXED = Instant.parse("2026-09-29T00:00:00Z");
    @Test void normalizesOriginAndDoesNotRetainRawPaths() {
        HttpObservation item = HttpObservation.from("GET", "LAB.INVALID", 8080, false, "/sensitive/person123", FIXED, 0);
        assertEquals("http://lab.invalid:8080", item.origin());
        assertFalse(item.toJson().contains("person123"));
        assertTrue(item.toJson().contains("normalized-metadata"));
        assertTrue(item.toJson().contains("burp-project-scope"));
    }
    @Test void rejectsMalformedAndQueryInputs() {
        assertThrows(IllegalArgumentException.class, () -> HttpObservation.from("GET", "lab.invalid", 8080, false, "/?token=secret", FIXED, 0));
        assertThrows(IllegalArgumentException.class, () -> HttpObservation.from("GET\r\n", "lab.invalid", 8080, false, "/", FIXED, 0));
        assertThrows(IllegalArgumentException.class, () -> HttpObservation.from("GET", "bad host", 8080, false, "/", FIXED, 0));
        assertThrows(IllegalArgumentException.class, () -> HttpObservation.from("GET", "lab.invalid", 0, false, "/", FIXED, 0));
    }
    @Test void identityIsStableAcrossClockAndSelectionButMethodMatters() {
        var first = HttpObservation.from("GET", "lab.invalid", 8080, false, "/health", FIXED, 0);
        var second = HttpObservation.from("GET", "lab.invalid", 8080, false, "/health", FIXED.plusSeconds(1), 3);
        var different = HttpObservation.from("POST", "lab.invalid", 8080, false, "/health", FIXED, 0);
        assertEquals(first.id(), second.id()); assertNotEquals(first.id(), different.id());
    }
    @Test void jsonEscapesControlCharacters() { assertEquals("\"a\\\"b\\\\c\\u000a\"", HttpObservation.quote("a\"b\\c\n")); }
    @Test void matchesSharedGoldenFixture() throws Exception {
        String actual = "{\"schemaVersion\":\"1.1.0\",\"observations\":[" + HttpObservation.from("GET", "lab.invalid", 8080, false, "/health", FIXED, 0).toJson() + "]}";
        String expected = java.nio.file.Files.readString(java.nio.file.Path.of("../../fixtures/http/golden.json")).trim();
        assertEquals(expected, actual);
    }
}
