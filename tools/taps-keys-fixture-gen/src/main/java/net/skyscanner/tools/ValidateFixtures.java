package net.skyscanner.tools;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import java.lang.reflect.Field;
import java.nio.file.Files;
import java.nio.file.Path;
import java.time.LocalDate;
import net.skyscanner.taps.keys.Key;
import net.skyscanner.taps.keys.KeyBuilder;
import net.skyscanner.taps.keys.KeySchema;
import net.skyscanner.taps.keys.Keys;

/**
 * L4 validator: reads golden_encodings.json, rebuilds each key using the production taps-keys
 * Java library, and verifies that every stored field matches what the library actually produces.
 *
 * <p>This cross-checks the fixture generator output against the real library independently of how
 * the fixtures were originally produced.
 *
 * <p>Usage: ./gradlew validateFixtures [-PencodingsFile=/path/to/golden_encodings.json]
 */
public class ValidateFixtures {

    public static void main(String[] args) throws Exception {
        String encodingsPath =
                args.length > 0
                        ? args[0]
                        : "/tmp/taps-keys-fixtures/golden_encodings.json";

        JsonArray encodings =
                JsonParser.parseString(Files.readString(Path.of(encodingsPath))).getAsJsonArray();

        int passed = 0;
        int failed = 0;

        for (JsonElement el : encodings) {
            JsonObject entry = el.getAsJsonObject();
            String schemaName = entry.get("schema").getAsString();
            String prefix = entry.get("prefix").getAsString();
            String inputSet = entry.get("input_set").getAsString();

            KeySchema schema = lookupSchema(prefix, schemaName);

            // Set L stores per-component route nodes; all others use a single value for all three
            int origAirport = entry.has("origin_airport")
                    ? entry.get("origin_airport").getAsInt() : entry.get("origin").getAsInt();
            int origCity    = entry.has("origin_city")
                    ? entry.get("origin_city").getAsInt()    : origAirport;
            int origCountry = entry.has("origin_country")
                    ? entry.get("origin_country").getAsInt() : origAirport;
            int destAirport = entry.has("destination_airport")
                    ? entry.get("destination_airport").getAsInt() : entry.get("destination").getAsInt();
            int destCity    = entry.has("destination_city")
                    ? entry.get("destination_city").getAsInt()    : destAirport;
            int destCountry = entry.has("destination_country")
                    ? entry.get("destination_country").getAsInt() : destAirport;
            int carrier = entry.get("carrier").getAsInt();
            LocalDate outbound = LocalDate.parse(entry.get("outbound_date").getAsString());
            LocalDate inbound =
                    entry.get("inbound_date").isJsonNull()
                            ? null
                            : LocalDate.parse(entry.get("inbound_date").getAsString());

            JsonElement isDirectEl = entry.get("is_direct");
            boolean wildcard =
                    isDirectEl.isJsonPrimitive()
                            && isDirectEl.getAsJsonPrimitive().isString()
                            && isDirectEl.getAsString().equals("wildcard");

            Key key;
            try {
                key = buildKey(schema, origAirport, origCity, origCountry,
                        destAirport, destCity, destCountry,
                        carrier, outbound, inbound, wildcard,
                        wildcard ? false : isDirectEl.getAsBoolean());
            } catch (Exception e) {
                System.out.printf(
                        "FAIL [%s/%s]: key build threw %s: %s%n",
                        schemaName, inputSet, e.getClass().getSimpleName(), e.getMessage());
                failed++;
                continue;
            }

            String expectedEncoded = entry.get("encoded_key").getAsString();
            String expectedToString = entry.get("to_string").getAsString();
            String expectedToStringPipe = entry.get("to_string_pipe").getAsString();
            String expectedSchemaToString = entry.get("schema_to_string").getAsString();
            int expectedEncodedLength = entry.get("encoded_length").getAsInt();
            String expectedOpenJawFilter = entry.get("open_jaw_filter").getAsString();

            boolean ok = true;

            if (!key.encode().equals(expectedEncoded)) {
                System.out.printf(
                        "FAIL [%s/%s]: encoded_key got %s expected %s%n",
                        schemaName, inputSet, key.encode(), expectedEncoded);
                ok = false;
            }
            if (!key.toString().equals(expectedToString)) {
                System.out.printf(
                        "FAIL [%s/%s]: to_string got %s expected %s%n",
                        schemaName, inputSet, key.toString(), expectedToString);
                ok = false;
            }
            if (!key.toString('|').equals(expectedToStringPipe)) {
                System.out.printf(
                        "FAIL [%s/%s]: to_string_pipe got %s expected %s%n",
                        schemaName, inputSet, key.toString('|'), expectedToStringPipe);
                ok = false;
            }
            if (!schema.toString().equals(expectedSchemaToString)) {
                System.out.printf(
                        "FAIL [%s/%s]: schema_to_string got %s expected %s%n",
                        schemaName, inputSet, schema.toString(), expectedSchemaToString);
                ok = false;
            }
            if (schema.encodedLength() != expectedEncodedLength) {
                System.out.printf(
                        "FAIL [%s/%s]: encoded_length got %d expected %d%n",
                        schemaName, inputSet, schema.encodedLength(), expectedEncodedLength);
                ok = false;
            }
            if (!schema.getOpenJawFilter().name().equals(expectedOpenJawFilter)) {
                System.out.printf(
                        "FAIL [%s/%s]: open_jaw_filter got %s expected %s%n",
                        schemaName, inputSet, schema.getOpenJawFilter().name(),
                        expectedOpenJawFilter);
                ok = false;
            }

            if (ok) {
                passed++;
            } else {
                failed++;
            }
        }

        System.out.printf("%nL4: %d/%d passed%n", passed, passed + failed);
        if (failed > 0) {
            System.exit(1);
        }
    }

    static KeySchema lookupSchema(String prefix, String name) throws Exception {
        Class<?> ns = prefix.equals("oneway") ? Keys.OneWay.class : Keys.Return.class;
        Field field = ns.getDeclaredField(name);
        return (KeySchema) field.get(null);
    }

    static Key buildKey(
            KeySchema schema,
            int origAirport, int origCity, int origCountry,
            int destAirport, int destCity, int destCountry,
            int carrier,
            LocalDate outbound,
            LocalDate inbound,
            boolean wildcard,
            boolean isDirect) {
        KeyBuilder b =
                schema.keyBuilder()
                        .marketingCarrier(carrier)
                        .originAirport(origAirport)
                        .originCity(origCity)
                        .originCountry(origCountry)
                        .destinationAirport(destAirport)
                        .destinationCity(destCity)
                        .destinationCountry(destCountry)
                        .outboundDepartureYearMonth(outbound)
                        .outboundDepartureDay(outbound);
        if (inbound != null) {
            b = b.inboundDepartureYearMonth(inbound).inboundDepartureDay(inbound);
        }
        b = wildcard ? b.isDirect(KeyBuilder.anyDirect()) : b.isDirect(isDirect);
        return b.build();
    }
}
