package net.skyscanner.tools;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import java.io.FileWriter;
import java.lang.reflect.Field;
import java.lang.reflect.Modifier;
import java.time.LocalDate;
import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import net.skyscanner.taps.keys.Key;
import net.skyscanner.taps.keys.KeyBuilder;
import net.skyscanner.taps.keys.KeySchema;
import net.skyscanner.taps.keys.Keys;
import net.skyscanner.taps.keys.encoding.KeyComponentSignature;

public class FixtureGenerator {

    /** Total input sets per schema — update this when adding new sets. */
    static final int SETS_PER_SCHEMA = 15;

    // Input Set A: Standard (same as KeyTest.java)
    static final int ORIG_A = 13554;
    static final int DEST_A = 13555;
    static final int CARRIER_A = -32480;
    static final LocalDate OUTBOUND_A = LocalDate.parse("2018-07-08");
    static final LocalDate INBOUND_A = LocalDate.parse("2018-08-25");
    static final boolean DIRECT_A = true;

    // Input Set B: Boundary minimums/maximums
    static final int ORIG_B = 0;
    static final int DEST_B = 65535;
    static final int CARRIER_B = -32768;
    static final LocalDate OUTBOUND_B = LocalDate.parse("1970-01-01");
    static final LocalDate INBOUND_B = LocalDate.parse("1970-01-01");
    static final boolean DIRECT_B = true;

    // Input Set C: Arbitrary non-special values
    static final int ORIG_C = 1;
    static final int DEST_C = 2;
    static final int CARRIER_C = 100;
    static final LocalDate OUTBOUND_C = LocalDate.parse("2025-12-31");
    static final LocalDate INBOUND_C = LocalDate.parse("2026-01-15");
    static final boolean DIRECT_C = true;

    // Input Set D: isDirect=false
    static final int ORIG_D = 13554;
    static final int DEST_D = 13555;
    static final int CARRIER_D = -32480;
    static final LocalDate OUTBOUND_D = LocalDate.parse("2018-07-08");
    static final LocalDate INBOUND_D = LocalDate.parse("2018-08-25");
    static final boolean DIRECT_D = false;

    // Input Set F: Base-32 carry points (origin=32, dest=1024, carrier=0 offset boundary)
    static final int ORIG_F = 32;
    static final int DEST_F = 1024;
    static final int CARRIER_F = 0;
    static final LocalDate OUTBOUND_F = LocalDate.parse("1972-09-15");
    static final LocalDate INBOUND_F = LocalDate.parse("1972-09-20");
    static final boolean DIRECT_F = true;

    // Input Set G: YEARMONTH overflow (value 1024 → 3-char "100")
    static final int ORIG_G = 13554;
    static final int DEST_G = 13555;
    static final int CARRIER_G = -32480;
    static final LocalDate OUTBOUND_G = LocalDate.parse("2055-05-15");
    static final LocalDate INBOUND_G = LocalDate.parse("2055-05-20");
    static final boolean DIRECT_G = true;

    // Input Set H: YEARMONTH just under overflow (value 1023 → "vv")
    static final int ORIG_H = 13554;
    static final int DEST_H = 13555;
    static final int CARRIER_H = -32480;
    static final LocalDate OUTBOUND_H = LocalDate.parse("2055-04-15");
    static final LocalDate INBOUND_H = LocalDate.parse("2055-04-20");
    static final boolean DIRECT_H = true;

    // Input Set I: Carrier max + airport midpoint + leap day
    static final int ORIG_I = 32768;
    static final int DEST_I = 32767;
    static final int CARRIER_I = 32767;
    static final LocalDate OUTBOUND_I = LocalDate.parse("2024-02-29");
    static final LocalDate INBOUND_I = LocalDate.parse("2024-03-01");
    static final boolean DIRECT_I = true;

    // Input Set J: Carrier near zero + route node max
    static final int ORIG_J = 65535;
    static final int DEST_J = 65536;
    static final int CARRIER_J = -1;
    static final LocalDate OUTBOUND_J = LocalDate.parse("1970-02-01");
    static final LocalDate INBOUND_J = LocalDate.parse("1970-01-15");
    static final boolean DIRECT_J = false;

    // Input Set K: Leap day + small values
    static final int ORIG_K = 1;
    static final int DEST_K = 1;
    static final int CARRIER_K = 1;
    static final LocalDate OUTBOUND_K = LocalDate.parse("2000-02-29");
    static final LocalDate INBOUND_K = LocalDate.parse("2000-02-28");
    static final boolean DIRECT_K = true;

    // Input Set L: Real Skyscanner node IDs — airport ≠ city ≠ country (LHR→JFK)
    // Production always passes three different hierarchy values per side.
    // Uses canonical test constants from quote-aggregator TestData.java.
    static final int ORIG_AIRPORT_L = 13554; // LHR
    static final int ORIG_CITY_L    = 4698;  // London
    static final int ORIG_COUNTRY_L = 247;   // UK
    static final int DEST_AIRPORT_L = 12712; // JFK
    static final int DEST_CITY_L    = 5772;  // NYC
    static final int DEST_COUNTRY_L = 115;   // US
    static final int CARRIER_L      = -12345; // production test carrier
    static final LocalDate OUTBOUND_L = LocalDate.parse("2020-06-15");
    static final LocalDate INBOUND_L  = LocalDate.parse("2020-07-20");
    static final boolean DIRECT_L = true;

    // Input Set Q: Mixed YEARMONTH overflow — outbound overflows (≥1024), inbound does not
    // Catches Python ports that apply the 3-char overflow path only to outbound.
    static final int ORIG_Q = 13554;
    static final int DEST_Q = 13555;
    static final int CARRIER_Q = -32480;
    static final LocalDate OUTBOUND_Q = LocalDate.parse("2055-05-15"); // ym=1024, overflows → 3 chars
    static final LocalDate INBOUND_Q  = LocalDate.parse("2018-07-08"); // ym=582, no overflow → 2 chars
    static final boolean DIRECT_Q = true;

    // Input Set M: Year-end / new-year rollover
    static final int ORIG_M = 13554;
    static final int DEST_M = 13555;
    static final int CARRIER_M = 200;
    static final LocalDate OUTBOUND_M = LocalDate.parse("2023-12-31");
    static final LocalDate INBOUND_M  = LocalDate.parse("2024-01-01");
    static final boolean DIRECT_M = true;

    // Input Set N: Same-day trip (outbound == inbound)
    static final int ORIG_N = 13554;
    static final int DEST_N = 13555;
    static final int CARRIER_N = -32480;
    static final LocalDate OUTBOUND_N = LocalDate.parse("2022-06-15");
    static final LocalDate INBOUND_N  = LocalDate.parse("2022-06-15");
    static final boolean DIRECT_N = false;

    // Signature probe schemas
    static final List<KeyComponentSignature> ORIGIN_AIRPORT_SIG =
        KeySchema.builder("").originAirport().build().signature();
    static final List<KeyComponentSignature> DEST_AIRPORT_SIG =
        KeySchema.builder("").destinationAirport().build().signature();
    static final List<KeyComponentSignature> OUTBOUND_YM_SIG =
        KeySchema.builder("").outboundDepartureYearMonth().build().signature();
    static final List<KeyComponentSignature> OUTBOUND_DAY_SIG =
        KeySchema.builder("").outboundDepartureDay().build().signature();
    static final List<KeyComponentSignature> INBOUND_YM_SIG =
        KeySchema.builder("").inboundDepartureYearMonth().build().signature();
    static final List<KeyComponentSignature> INBOUND_DAY_SIG =
        KeySchema.builder("").inboundDepartureDay().build().signature();

    public static void main(String[] args) throws Exception {
        List<Map<String, Object>> encodings = new ArrayList<>();
        List<Map<String, Object>> signatures = new ArrayList<>();
        List<String> skipped = new ArrayList<>();

        int onewayCount = 0;
        for (Field f : Keys.OneWay.class.getDeclaredFields()) {
            if (Modifier.isPublic(f.getModifiers()) && Modifier.isStatic(f.getModifiers())
                    && f.getType() == KeySchema.class) {
                KeySchema schema = (KeySchema) f.get(null);
                String name = f.getName();
                int before = encodings.size();
                generateEncodings(encodings, name, "oneway", schema);
                if (encodings.size() - before < SETS_PER_SCHEMA) {
                    skipped.add("oneway." + name + " (" + (encodings.size() - before) + "/" + SETS_PER_SCHEMA + " sets)");
                }
                generateSignatures(signatures, name, schema);
                onewayCount++;
            }
        }

        int returnCount = 0;
        for (Field f : Keys.Return.class.getDeclaredFields()) {
            if (Modifier.isPublic(f.getModifiers()) && Modifier.isStatic(f.getModifiers())
                    && f.getType() == KeySchema.class) {
                KeySchema schema = (KeySchema) f.get(null);
                String name = f.getName();
                int before = encodings.size();
                generateEncodings(encodings, name, "return", schema);
                if (encodings.size() - before < SETS_PER_SCHEMA) {
                    skipped.add("return." + name + " (" + (encodings.size() - before) + "/" + SETS_PER_SCHEMA + " sets)");
                }
                generateSignatures(signatures, name, schema);
                returnCount++;
            }
        }

        Gson gson = new GsonBuilder().setPrettyPrinting().create();

        try (FileWriter w = new FileWriter("golden_encodings.json")) {
            gson.toJson(encodings, w);
        }
        try (FileWriter w = new FileWriter("golden_signatures.json")) {
            gson.toJson(signatures, w);
        }

        System.out.println("Schemas found: " + onewayCount + " oneway + " + returnCount + " return = " + (onewayCount + returnCount) + " total");
        System.out.println("Generated " + encodings.size() + " encoding fixtures (expected " + (onewayCount + returnCount) * SETS_PER_SCHEMA + ")");
        System.out.println("Generated " + signatures.size() + " signature fixtures (expected " + (onewayCount + returnCount) + ")");
        if (!skipped.isEmpty()) {
            System.err.println("WARNING: " + skipped.size() + " schemas had incomplete fixture sets:");
            for (String s : skipped) System.err.println("  - " + s);
        }

        // Fail hard if counts don't match — don't let broken fixtures propagate
        int expectedSigs = onewayCount + returnCount;
        int expectedEncs = expectedSigs * SETS_PER_SCHEMA;
        if (signatures.size() != expectedSigs || encodings.size() != expectedEncs) {
            System.err.println("FATAL: Fixture count mismatch. Expected " + expectedEncs + " encodings and " + expectedSigs + " signatures.");
            System.exit(1);
        }
    }

    static void generateEncodings(List<Map<String, Object>> out, String name,
            String prefix, KeySchema schema) {
        Object[][] sets = {
            {"A", ORIG_A, DEST_A, CARRIER_A, OUTBOUND_A, INBOUND_A, DIRECT_A},
            {"B", ORIG_B, DEST_B, CARRIER_B, OUTBOUND_B, INBOUND_B, DIRECT_B},
            {"C", ORIG_C, DEST_C, CARRIER_C, OUTBOUND_C, INBOUND_C, DIRECT_C},
            {"D", ORIG_D, DEST_D, CARRIER_D, OUTBOUND_D, INBOUND_D, DIRECT_D},
            {"F", ORIG_F, DEST_F, CARRIER_F, OUTBOUND_F, INBOUND_F, DIRECT_F},
            {"G", ORIG_G, DEST_G, CARRIER_G, OUTBOUND_G, INBOUND_G, DIRECT_G},
            {"H", ORIG_H, DEST_H, CARRIER_H, OUTBOUND_H, INBOUND_H, DIRECT_H},
            {"I", ORIG_I, DEST_I, CARRIER_I, OUTBOUND_I, INBOUND_I, DIRECT_I},
            {"J", ORIG_J, DEST_J, CARRIER_J, OUTBOUND_J, INBOUND_J, DIRECT_J},
            {"K", ORIG_K, DEST_K, CARRIER_K, OUTBOUND_K, INBOUND_K, DIRECT_K},
            {"M", ORIG_M, DEST_M, CARRIER_M, OUTBOUND_M, INBOUND_M, DIRECT_M},
            {"N", ORIG_N, DEST_N, CARRIER_N, OUTBOUND_N, INBOUND_N, DIRECT_N},
            {"Q", ORIG_Q, DEST_Q, CARRIER_Q, OUTBOUND_Q, INBOUND_Q, DIRECT_Q},
        };

        for (Object[] set : sets) {
            try {
                Key key = buildKey(schema, (int) set[1], (int) set[2], (int) set[3],
                        (LocalDate) set[4], (LocalDate) set[5], (boolean) set[6]);
                Map<String, Object> entry = new LinkedHashMap<>();
                entry.put("schema", name);
                entry.put("prefix", prefix);
                entry.put("input_set", (String) set[0]);
                entry.put("origin", set[1]);
                entry.put("destination", set[2]);
                entry.put("carrier", set[3]);
                entry.put("outbound_date", set[4].toString());
                entry.put("inbound_date", set[5].toString());
                entry.put("is_direct", set[6]);
                entry.put("encoded_key", key.encode());
                entry.put("to_string", key.toString());
                entry.put("to_string_pipe", key.toString('|'));
                entry.put("schema_to_string", schema.toString());
                entry.put("encoded_length", schema.encodedLength());
                entry.put("open_jaw_filter", schema.getOpenJawFilter().name());
                out.add(entry);
            } catch (Exception e) {
                System.err.println("WARN: Set " + set[0] + " failed for " + name + ": " + e.getMessage());
            }
        }

        // Set L: Different values per route node (airport ≠ city ≠ country)
        try {
            Key key = buildKeyPerComponent(schema,
                    ORIG_AIRPORT_L, ORIG_CITY_L, ORIG_COUNTRY_L,
                    DEST_AIRPORT_L, DEST_CITY_L, DEST_COUNTRY_L,
                    CARRIER_L, OUTBOUND_L, INBOUND_L, DIRECT_L);
            Map<String, Object> entry = new LinkedHashMap<>();
            entry.put("schema", name);
            entry.put("prefix", prefix);
            entry.put("input_set", "L");
            entry.put("origin_airport", ORIG_AIRPORT_L);
            entry.put("origin_city",    ORIG_CITY_L);
            entry.put("origin_country", ORIG_COUNTRY_L);
            entry.put("destination_airport", DEST_AIRPORT_L);
            entry.put("destination_city",    DEST_CITY_L);
            entry.put("destination_country", DEST_COUNTRY_L);
            entry.put("carrier", CARRIER_L);
            entry.put("outbound_date", OUTBOUND_L.toString());
            entry.put("inbound_date", INBOUND_L.toString());
            entry.put("is_direct", DIRECT_L);
            entry.put("encoded_key", key.encode());
            entry.put("to_string", key.toString());
            entry.put("to_string_pipe", key.toString('|'));
            entry.put("schema_to_string", schema.toString());
            entry.put("encoded_length", schema.encodedLength());
            entry.put("open_jaw_filter", schema.getOpenJawFilter().name());
            out.add(entry);
        } catch (Exception e) {
            System.err.println("WARN: Set L failed for " + name + ": " + e.getMessage());
        }

        // Set E: trailing wildcard (anyDirect)
        try {
            Key key = buildKeyWithWildcard(schema, ORIG_A, DEST_A, CARRIER_A, OUTBOUND_A, INBOUND_A);
            Map<String, Object> entry = new LinkedHashMap<>();
            entry.put("schema", name);
            entry.put("prefix", prefix);
            entry.put("input_set", "E");
            entry.put("origin", ORIG_A);
            entry.put("destination", DEST_A);
            entry.put("carrier", CARRIER_A);
            entry.put("outbound_date", OUTBOUND_A.toString());
            entry.put("inbound_date", INBOUND_A.toString());
            entry.put("is_direct", "wildcard");
            entry.put("encoded_key", key.encode());
            entry.put("to_string", key.toString());
            entry.put("to_string_pipe", key.toString('|'));
            entry.put("schema_to_string", schema.toString());
            entry.put("encoded_length", schema.encodedLength());
            entry.put("open_jaw_filter", schema.getOpenJawFilter().name());
            out.add(entry);
        } catch (Exception e) {
            System.err.println("WARN: Set E failed for " + name + ": " + e.getMessage());
        }
    }

    static Key buildKeyPerComponent(KeySchema schema,
            int origAirport, int origCity, int origCountry,
            int destAirport, int destCity, int destCountry,
            int carrier, LocalDate outbound, LocalDate inbound, boolean isDirect) {
        return schema.keyBuilder()
                .marketingCarrier(carrier)
                .originAirport(origAirport)
                .originCity(origCity)
                .originCountry(origCountry)
                .destinationAirport(destAirport)
                .destinationCity(destCity)
                .destinationCountry(destCountry)
                .outboundDepartureYearMonth(outbound)
                .outboundDepartureDay(outbound)
                .inboundDepartureYearMonth(inbound)
                .inboundDepartureDay(inbound)
                .isDirect(isDirect)
                .build();
    }

    static Key buildKey(KeySchema schema, int orig, int dest, int carrier,
            LocalDate outbound, LocalDate inbound, boolean isDirect) {
        return schema.keyBuilder()
                .marketingCarrier(carrier)
                .originAirport(orig)
                .originCity(orig)
                .originCountry(orig)
                .destinationAirport(dest)
                .destinationCity(dest)
                .destinationCountry(dest)
                .outboundDepartureYearMonth(outbound)
                .outboundDepartureDay(outbound)
                .inboundDepartureYearMonth(inbound)
                .inboundDepartureDay(inbound)
                .isDirect(isDirect)
                .build();
    }

    static Key buildKeyWithWildcard(KeySchema schema, int orig, int dest, int carrier,
            LocalDate outbound, LocalDate inbound) {
        return schema.keyBuilder()
                .marketingCarrier(carrier)
                .originAirport(orig)
                .originCity(orig)
                .originCountry(orig)
                .destinationAirport(dest)
                .destinationCity(dest)
                .destinationCountry(dest)
                .outboundDepartureYearMonth(outbound)
                .outboundDepartureDay(outbound)
                .inboundDepartureYearMonth(inbound)
                .inboundDepartureDay(inbound)
                .isDirect(KeyBuilder.anyDirect())
                .build();
    }

    static void generateSignatures(List<Map<String, Object>> out, String name, KeySchema schema) {
        Map<String, Object> entry = new LinkedHashMap<>();
        entry.put("schema", name);
        entry.put("schema_to_string", schema.toString());
        entry.put("origin_airport_disjoint",
                Collections.disjoint(schema.signature(), ORIGIN_AIRPORT_SIG));
        entry.put("destination_airport_disjoint",
                Collections.disjoint(schema.signature(), DEST_AIRPORT_SIG));
        entry.put("outbound_year_month_disjoint",
                Collections.disjoint(schema.signature(), OUTBOUND_YM_SIG));
        entry.put("outbound_day_disjoint",
                Collections.disjoint(schema.signature(), OUTBOUND_DAY_SIG));
        entry.put("inbound_year_month_disjoint",
                Collections.disjoint(schema.signature(), INBOUND_YM_SIG));
        entry.put("inbound_day_disjoint",
                Collections.disjoint(schema.signature(), INBOUND_DAY_SIG));
        out.add(entry);
    }
}