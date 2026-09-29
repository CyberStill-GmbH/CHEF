package chef;

import java.net.URI;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.time.Instant;
import java.util.HexFormat;
import java.util.Locale;

/** Pure mapper. Does not retain Burp messages, credentials, query strings or raw paths. */
public record HttpObservation(String id, String method, String origin, String pathSha256,
                              String metadataSha256, String observedAt, String locator) {
    public static HttpObservation from(String method, String host, int port, boolean secure,
                                       String pathWithoutQuery, Instant observedAt, int selectionIndex) {
        if (method == null || !method.matches("[A-Z]{1,16}") || host == null || host.length() > 253 ||
            port < 1 || port > 65535 || pathWithoutQuery == null || pathWithoutQuery.length() > 8192 ||
            !pathWithoutQuery.startsWith("/") || pathWithoutQuery.contains("?") || pathWithoutQuery.contains("#") ||
            selectionIndex < 0 || observedAt == null) {
            throw new IllegalArgumentException("Invalid normalized HTTP metadata");
        }
        try {
            String normalizedHost = host.toLowerCase(Locale.ROOT);
            URI endpoint = new URI(secure ? "https" : "http", null, normalizedHost, port, null, null, null);
            if (endpoint.getHost() == null || normalizedHost.chars().anyMatch(c -> c > 127 || c < 33)) {
                throw new IllegalArgumentException("Invalid HTTP origin");
            }
            String origin = endpoint.toASCIIString();
            String pathHash = sha256(pathWithoutQuery);
            String metadata = "[" + quote(method) + "," + quote(origin) + "," + quote(pathHash) + "]";
            String identity = "[\"http-endpoint/v1\"," + quote(method) + "," + quote(origin) + "," + quote(pathHash) + "]";
            return new HttpObservation("observation:" + sha256(identity), method, origin, pathHash,
                sha256(metadata), observedAt.toString(), "burp-selection:" + selectionIndex);
        } catch (java.net.URISyntaxException exception) {
            throw new IllegalArgumentException("Invalid HTTP origin");
        }
    }

    public String toJson() {
        return "{\"id\":" + quote(id) + ",\"source\":\"burp-selection\",\"fact\":\"http-endpoint\"," +
            "\"method\":" + quote(method) + ",\"origin\":" + quote(origin) + ",\"pathSha256\":" + quote(pathSha256) +
            ",\"observedAt\":" + quote(observedAt) + ",\"assertion\":\"observed\",\"confidence\":1," +
            "\"evidence\":{\"digestKind\":\"normalized-metadata\",\"sha256\":" + quote(metadataSha256) +
            ",\"locator\":" + quote(locator) + "},\"scopePolicyId\":\"burp-project-scope\"}";
    }

    static String quote(String value) {
        StringBuilder output = new StringBuilder("\"");
        for (int index = 0; index < value.length(); index++) {
            char item = value.charAt(index);
            if (item == '"' || item == '\\') output.append('\\').append(item);
            else if (item < 32) output.append(String.format(Locale.ROOT, "\\u%04x", (int) item));
            else output.append(item);
        }
        return output.append('"').toString();
    }

    static String sha256(String value) {
        try { return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(value.getBytes(StandardCharsets.UTF_8))); }
        catch (NoSuchAlgorithmException impossible) { throw new IllegalStateException("SHA-256 unavailable", impossible); }
    }
}
