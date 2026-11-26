#!/usr/bin/env bash
# Fetches the pinned projects of the Gradle evidence (docs/ai/design/gradle.md) into
# tests/conformance/corpus/gradle/<owner>__<repo> (gitignored): every `*.gradle` file (Groovy DSL: build, settings, init and
# script plugins; `.gradle.kts` is Kotlin and not matched) of 43 public projects. The list was written before any result:
# well known Gradle projects (JVM libraries, Android apps and libraries, Gradle plugins, Spring, Apache, Netflix, Elastic);
# a project joined only if its tree holds at least 4 `.gradle` files (the 2026-11-26 default-branch heads, pinned below).
# `scripts/evidence_gradle.py` caps a project at 60 files (the first by path), so that no project outweighs the others.
export TARBALL_LENIENT=1 TARBALL_OWNER=1
source "$(dirname "$0")/tarball-corpus.sh"
P='*.gradle !*/node_modules/* !*/vendor/* !*/build/* !*/.git/*'
for line in \
  'spring-projects/spring-boot ebb5e11c70c50469c45f8aada56dd6d477c6b77f $P' \
  'spring-projects/spring-framework 9a9585e39882b9411be27d6c039787d9d48fe7e7 $P' \
  'spring-projects/spring-security 187895ce347cb342820b9f9eccbd594baad970d2 $P' \
  'gradle/gradle 80e3397340ca2cdabb3a4f429ae3a7baa7f755e9 $P' \
  'gradle/native-platform d1f6d86e8aa422a33a85a1ea30b628047f3e87ac $P' \
  'apache/groovy 40d90fabe32f4d38aa60f1ed410192260f844282 $P' \
  'apache/grails-core aa4de162ecc492d3334f6cc592626c0d6805e6de $P' \
  'elastic/elasticsearch 569b2e851c88ea54fba742d2be4c6127b895a933 $P' \
  'apache/kafka a0f06c4a80219955032bc3da9744e6fac373a39c $P' \
  'apache/lucene 77d56fefb355652a56d475449268d2ca1844675f $P' \
  'apache/solr 91ef2d0d501519b3480664b671c99d0deabc4bd5 $P' \
  'apache/beam fcb5fcc56b6de0dbf97d804a10961dda12513cfe $P' \
  'apache/ofbiz-framework 681528ab75655624824c44fba04cae465475b74c $P' \
  'hibernate/hibernate-orm 9f06537941a2e875b0e4657edb8fb9fc880c7e46 $P' \
  'micronaut-projects/micronaut-core e63b92c178b05bc16a590bf53d94937ab1c5e91b $P' \
  'spockframework/spock 3ff35eb838823f87510ad61b8095c75d1566a305 $P' \
  'testcontainers/testcontainers-java 9f7ada889f53e97f8358806c18deb7c25dda66f7 $P' \
  'ReactiveX/RxAndroid 311e44531ae7e795fcba14af64f6ad2280cf2205 $P' \
  'lysine-dev/retrofit 54c204719ac5bf952402aaf81f54f9a2f24beee2 $P' \
  'bumptech/glide aac5418e7b2bcda62f9206dc5c9c896eab75687c $P' \
  'airbnb/lottie-android 05ea92e90381eb8a8ae06855ea2b74f322bebbec $P' \
  'greenrobot/EventBus 0194926b3bcf70cc0d7bfd3c5da16708dd5ab876 $P' \
  'google/ExoPlayer dd430f7053a1a3958deea3ead6a0565150c06bfc $P' \
  'Netflix/conductor 548f386c2c6c1e4d467b8551edbb6182881004ae $P' \
  'Netflix/eureka 793de0a7e7f8200e4da45a08217a5a8b811f1b0a $P' \
  'Netflix/zuul 908894af8cae9a2aed27761a5c8cde8672fdc32d $P' \
  'Netflix/Hystrix 5ce3bc58c38e7ca60ef2fe0e516e390e294ad941 $P' \
  'libgdx/libgdx be635906dc2a437450403bd88b7a40fc8d944354 $P' \
  'react/react-native 0c688a79c9359021d98f849bdbd43a0fc100cea7 $P' \
  'meta/fresco e7c3a39e3d7a04ed2fa312d49dc11ba2cf242f20 $P' \
  'meta/litho 5893542388f74382113c5e58529b5ec669478a43 $P' \
  'gradle/gradle-build-action 093dfe9d598ec5a42246855d09b49dc76803c005 $P' \
  'gradle/develocity-build-config-samples affd226eb7e8c84cf910382ed0bdb82a865559fd $P' \
  'sqldelight/sqldelight 74b446bc5bd960f238f681372aba0960714f810b $P' \
  'google/dagger b38d3564bb856367ccfb13353ef912d79af39221 $P' \
  'ratpack/ratpack cc0e5a87474154cdab9b86397cb72f2ede39757b $P' \
  'SonarSource/sonarqube 9b2ac69c8d6b69c50404c6af9c5def107fed1d9d $P' \
  'MinecraftForge/MinecraftForge 6a06826b149192c72faca0bb68df35bc28238c18 $P' \
  'FabricMC/fabric-api 495da79f62942fa2630b576ce2075536d37b8223 $P' \
  'android/compose-samples c7a01f04dcd2010935a1e11bec3da5c59f55f6a2 $P' \
  'android/views-widgets-samples 2238cc873501f9cda63605051de11832bb736a8a $P' \
  'Kotlin/dokka c31976976fa37da1e31df6f6a0a98ea4f45f8bc3 $P' \
  'OpenAPITools/openapi-generator cad816d23e0edeb76a4ee4db936a1a727e3e042e $P' \
  ; do
  fetch_tarballs gradle <<<"${line//\$P/$P}" || echo "no .gradle file: ${line%% *}"
done
