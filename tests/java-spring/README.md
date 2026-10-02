# Generated models in Spring Boot

Run `./scripts/verify_spring_boot.sh` from the repository with Maven and a Java 25
JDK on `JAVA_HOME`/`PATH`. The runner copies this fixture to a temporary folder,
generates both model styles, runs Maven tests, and removes the temporary output.
It requires network access for uncached dependencies.

The fixture pins Spring Boot 4.1.1 and uses its Jackson 3 dependency management.
It uses actual embedded-server HTTP requests to test Spring JSON/XML converters
and Jakarta validation, plus standalone mapper and direct StAX round trips.
No Jackson 2 databind/XML dependencies or native PolyXML library are used.

Records with simple content and attributes require
`XmlMapperBuilderCustomizer` with `nameForTextElement("value")`.
The same configuration is used for standalone XML mapping. See the Java guide
for application setup and the existing schema/runtime limits.

The main CI workflow runs this same command in a dedicated Java 25 job.
