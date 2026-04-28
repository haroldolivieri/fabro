package net.skyscanner.tools;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import java.io.FileWriter;
import java.lang.reflect.Field;
import java.lang.reflect.Modifier;
import java.time.LocalDate;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Random;
import net.skyscanner.taps.keys.Key;
import net.skyscanner.taps.keys.KeySchema;
import net.skyscanner.taps.keys.Keys;

/**
 * L6 fuzz encoder: generates random valid inputs for all 142 schemas, encodes each with the
 * production Java library, and writes results to a JSON file for comparison with Python.
 *
 * <p>Uses a fixed seed for reproducibility — same seed produces identical outputs across runs.
 *
 * <p>Usage: ./gradlew fuzzEncode [--seed 42] [--count 100] [--output /tmp/fuzz_java_outputs.json]
 */
public class FuzzEncoder {

    // YEARMONTH range: 1970-01 to 2055-04 keeps values in [0, 1023] (avoids overflow edge
    // case which is already covered by fixture Set G)
    private static final LocalDate MIN_DATE = LocalDate.of(1970, 1, 1);
    private static final LocalDate MAX_DATE = LocalDate.of(2055, 4, 28);
    private static final int DATE_RANGE_DAYS = (int) (MAX_DATE.toEpochDay() - MIN_DATE.toEpochDay());

    public static void main(String[] args) throws Exception {
        long seed = 42;
        int countPerSchema = 100;
        String outputPath = "/tmp/fuzz_java_outputs.json";

        for (int i = 0; i < args.length; i++) {
            switch (args[i]) {
                case "--seed":
                    seed = Long.parseLong(args[++i]);
                    break;
                case "--count":
                    countPerSchema = Integer.parseInt(args[++i]);
                    break;
                case "--output":
                    outputPath = args[++i];
                    break;
                default:
                    System.err.println("Unknown arg: " + args[i]);
                    System.exit(1);
            }
        }

        Random random = new Random(seed);
        List<Map<String, Object>> results = new ArrayList<>();
        int errors = 0;
        int schemaCount = 0;

        // Process OneWay schemas
        for (Field f : Keys.OneWay.class.getDeclaredFields()) {
            if (isSchemaField(f)) {
                KeySchema schema = (KeySchema) f.get(null);
                errors += generateFuzzEntries(results, f.getName(), "oneway", schema,
                        random, countPerSchema);
                schemaCount++;
            }
        }

        // Process Return schemas
        for (Field f : Keys.Return.class.getDeclaredFields()) {
            if (isSchemaField(f)) {
                KeySchema schema = (KeySchema) f.get(null);
                errors += generateFuzzEntries(results, f.getName(), "return", schema,
                        random, countPerSchema);
                schemaCount++;
            }
        }

        Gson gson = new GsonBuilder().setPrettyPrinting().create();
        try (FileWriter w = new FileWriter(outputPath)) {
            gson.toJson(results, w);
        }

        System.out.println("Fuzz: " + results.size() + " entries generated for "
                + schemaCount + " schemas (seed=" + seed + ", count=" + countPerSchema + ")");

        if (errors > 0) {
            System.err.println("FATAL: " + errors + " entries failed to encode");
            System.exit(1);
        }
    }

    private static boolean isSchemaField(Field f) {
        return Modifier.isPublic(f.getModifiers())
                && Modifier.isStatic(f.getModifiers())
                && f.getType() == KeySchema.class;
    }

    private static int generateFuzzEntries(List<Map<String, Object>> out, String name,
            String prefix, KeySchema schema, Random random, int count) {
        int errors = 0;
        for (int i = 0; i < count; i++) {
            int origin = random.nextInt(65537);       // [0, 65536]
            int destination = random.nextInt(65537);
            int carrier = random.nextInt(65536) - 32768; // [-32768, 32767]
            LocalDate outbound = randomDate(random);
            LocalDate inbound = randomDate(random);
            boolean isDirect = random.nextBoolean();

            try {
                Key key = FixtureGenerator.buildKey(schema, origin, destination, carrier,
                        outbound, inbound, isDirect);

                Map<String, Object> entry = new LinkedHashMap<>();
                entry.put("schema", name);
                entry.put("prefix", prefix);
                entry.put("fuzz_id", i);
                entry.put("origin", origin);
                entry.put("destination", destination);
                entry.put("carrier", carrier);
                entry.put("outbound_date", outbound.toString());
                entry.put("inbound_date", inbound.toString());
                entry.put("is_direct", isDirect);
                entry.put("encoded_key", key.encode());
                entry.put("to_string", key.toString());
                entry.put("to_string_pipe", key.toString('|'));
                out.add(entry);
            } catch (Exception e) {
                System.err.println("WARN: Fuzz entry " + i + " failed for " + name
                        + ": " + e.getMessage());
                errors++;
            }
        }
        return errors;
    }

    private static LocalDate randomDate(Random random) {
        long epochDay = MIN_DATE.toEpochDay() + random.nextInt(DATE_RANGE_DAYS + 1);
        return LocalDate.ofEpochDay(epochDay);
    }
}
