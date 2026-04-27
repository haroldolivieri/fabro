package net.skyscanner.tools;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import java.io.FileWriter;
import java.nio.file.Files;
import java.nio.file.Path;
import java.time.LocalDate;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import net.skyscanner.taps.keys.Key;
import net.skyscanner.taps.keys.KeyBuilder;
import net.skyscanner.taps.keys.KeySchema;

/**
 * L5 Java encoder: reads fixture inputs from golden_encodings.json, encodes each using the
 * production taps-keys Java library, and writes live results to an output JSON file.
 *
 * <p>Output contains only live Java-computed values — no expected values from the fixture JSON
 * are copied, so the result represents a pure Java encoding run for direct comparison with Python.
 *
 * <p>Usage: ./gradlew encodeMain
 *   [-PencodingsFile=/path/to/golden_encodings.json]
 *   [-PoutputFile=/tmp/java_outputs.json]
 */
public class EncodeMain {

    public static void main(String[] args) throws Exception {
        String encodingsPath =
                args.length > 0
                        ? args[0]
                        : "/tmp/taps-keys-fixtures/golden_encodings.json";
        String outputPath = args.length > 1 ? args[1] : "/tmp/java_outputs.json";

        JsonArray encodings =
                JsonParser.parseString(Files.readString(Path.of(encodingsPath))).getAsJsonArray();

        List<Map<String, Object>> results = new ArrayList<>();
        int errors = 0;

        for (JsonElement el : encodings) {
            JsonObject entry = el.getAsJsonObject();
            String schemaName = entry.get("schema").getAsString();
            String prefix = entry.get("prefix").getAsString();
            String inputSet = entry.get("input_set").getAsString();

            KeySchema schema = ValidateFixtures.lookupSchema(prefix, schemaName);

            int origin = entry.get("origin").getAsInt();
            int destination = entry.get("destination").getAsInt();
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
                key = ValidateFixtures.buildKey(
                        schema, origin, destination, carrier, outbound, inbound, wildcard,
                        wildcard ? false : isDirectEl.getAsBoolean());
            } catch (Exception e) {
                System.err.printf(
                        "ERROR [%s/%s]: %s: %s%n",
                        schemaName, inputSet, e.getClass().getSimpleName(), e.getMessage());
                errors++;
                continue;
            }

            Map<String, Object> result = new LinkedHashMap<>();
            result.put("schema", schemaName);
            result.put("prefix", prefix);
            result.put("input_set", inputSet);
            result.put("encoded_key", key.encode());
            result.put("to_string", key.toString());
            result.put("to_string_pipe", key.toString('|'));
            results.add(result);
        }

        Gson gson = new GsonBuilder().setPrettyPrinting().create();
        try (FileWriter w = new FileWriter(outputPath)) {
            gson.toJson(results, w);
        }

        System.out.printf("Java encoder: %d entries written to %s%n", results.size(), outputPath);
        if (errors > 0) {
            System.err.printf("FATAL: %d entries failed to encode%n", errors);
            System.exit(1);
        }
    }
}
