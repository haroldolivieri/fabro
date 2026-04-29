# taps-keys-fixture-gen

Java tool that generates and validates golden fixture data for the taps-keys Python port.

## Prerequisites

Requires access to Skyscanner's internal Artifactory to resolve the `net.skyscanner.taps-keys:taps-keys` dependency.

```bash
export SKYSCANNER_ARTIFACTORY_MAVEN_USER=<your-user>
export SKYSCANNER_ARTIFACTORY_MAVEN_PASSWORD=<your-password>
```

## Build the fat JAR

```bash
cd tools/taps-keys-fixture-gen
./gradlew shadowJar
# Output: build/libs/taps-keys-fixture-gen.jar
```

Set the `TAPS_KEYS_JAR` env var so Fabro workflows can find it:

```bash
export TAPS_KEYS_JAR=$(pwd)/tools/taps-keys-fixture-gen/build/libs/taps-keys-fixture-gen.jar
```

## Important

**Do not commit the JAR.** It bundles internal Skyscanner bytecode and must not appear in this public repo or its git history. The file is listed in `.gitignore`.
