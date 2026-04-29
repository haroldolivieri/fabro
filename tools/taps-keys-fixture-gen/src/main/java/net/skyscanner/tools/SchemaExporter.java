package net.skyscanner.tools;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import java.lang.reflect.Field;
import java.lang.reflect.Modifier;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import net.skyscanner.taps.keys.KeySchema;
import net.skyscanner.taps.keys.Keys;

/**
 * Exports all schema metadata from Keys.OneWay and Keys.Return as JSON.
 *
 * <p>Outputs a JSON array to stdout, one object per schema:
 * [{"name": "AIRPORT_AIRPORT_DAY_OW", "prefix": "oneway",
 *   "to_string": "oneway_originAirport_...", "encoded_length": 13,
 *   "open_jaw_filter": "NONE"}, ...]
 *
 * <p>Usage: java -cp taps-keys-fixture-gen.jar net.skyscanner.tools.SchemaExporter
 */
public class SchemaExporter {

    public static void main(String[] args) throws Exception {
        List<Map<String, Object>> schemas = new ArrayList<>();

        for (Field f : Keys.OneWay.class.getDeclaredFields()) {
            if (isSchemaField(f)) {
                schemas.add(export("oneway", f.getName(), (KeySchema) f.get(null)));
            }
        }

        for (Field f : Keys.Return.class.getDeclaredFields()) {
            if (isSchemaField(f)) {
                schemas.add(export("return", f.getName(), (KeySchema) f.get(null)));
            }
        }

        Gson gson = new GsonBuilder().setPrettyPrinting().create();
        System.out.println(gson.toJson(schemas));

        System.err.println("Exported " + schemas.size() + " schemas ("
                + schemas.stream().filter(s -> "oneway".equals(s.get("prefix"))).count() + " oneway, "
                + schemas.stream().filter(s -> "return".equals(s.get("prefix"))).count() + " return)");
    }

    private static boolean isSchemaField(Field f) {
        return Modifier.isPublic(f.getModifiers())
                && Modifier.isStatic(f.getModifiers())
                && f.getType() == KeySchema.class;
    }

    private static Map<String, Object> export(String prefix, String name, KeySchema schema) {
        Map<String, Object> entry = new LinkedHashMap<>();
        entry.put("name", name);
        entry.put("prefix", prefix);
        entry.put("to_string", schema.toString());
        entry.put("encoded_length", schema.encodedLength());
        entry.put("open_jaw_filter", schema.getOpenJawFilter().name());
        return entry;
    }
}
