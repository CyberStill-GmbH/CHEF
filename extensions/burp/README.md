# CHEF Montoya spike

Passive proof of concept: select up to100 in-scope requests → normalized HTTP observations with metadata digest and hashed path → inspect/copy JSON in a suite tab. No traffic is sent. No raw body, auth headers, query or raw path is retained. This is not a complete BApp and does not yet correlate Nmap with HTTP inside Burp.

Build with JDK21 and Maven3.9.x:

```sh
mvn -B -f extensions/burp/pom.xml clean verify
```

Load `extensions/burp/target/chef-montoya-spike-0.1.0.jar` manually in Burp Extensions. Montoya2026.7 is provided by Burp, not bundled in the JAR. Minimum Burp version has not been measured. First load/use/unload must be recorded in [manual tests](../../docs/bapp/manual-tests.md).

The wire envelope1.1.0 is a proposed HTTP contract distinct from the Nmap snapshot1.0.0; see [schema](../../docs/contracts/http-observation.schema.json). JVM and TypeScript tests share a golden fixture. Normalized metadata is not the raw request digest, and the observed timestamp is selection time. Scope uses the current Burp project, with no active discovery.

The worker queue is bounded, display updates run on EDT and unload stops work. Only bounded metadata is copied from selected messages; no long-term Burp request references are kept. Performance of that copy on EDT needs manual measurement before calling this responsive for all projects. Saving JSON through a proper export dialog, persistence and graph correlation are later increments.
