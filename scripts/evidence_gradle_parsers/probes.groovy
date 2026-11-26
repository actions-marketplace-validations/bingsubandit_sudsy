plugins { id 'java' }
//----
plugins { id 'org.springframework.boot' version '3.2.0' apply false }
//----
apply plugin: 'java'
//----
dependencies { implementation 'a:b:1.0' }
//----
dependencies { implementation 'a:b:1.0', 'c:d:2.0' }
//----
dependencies { implementation("a:b:1.0") { exclude group: 'x', module: 'y' } }
//----
dependencies { testImplementation(platform("org.junit:junit-bom:5.10.0")) }
//----
repositories { maven { url "https://repo.example.com" } }
//----
repositories { maven { url = uri("https://repo.example.com"); credentials { username = System.getenv('U') } } }
//----
ext { springVersion = '6.1.0'; libs = [guava: 'com.google.guava:guava:33.0.0-jre', junit: 'junit:junit:4.13'] }
//----
def v = "1.0-${System.currentTimeMillis()}"
//----
version = "${project.name}-$buildNumber"
//----
task hello { doLast { println "Hello ${project.name}" } }
//----
task copyDocs(type: Copy) { from 'src/docs'; into "$buildDir/docs" }
//----
tasks.register('zip', Zip) { archiveFileName = 'a.zip' }
//----
tasks.withType(JavaCompile).configureEach { options.encoding = 'UTF-8' }
//----
tasks.named('test') { useJUnitPlatform() }
//----
test { useJUnitPlatform(); testLogging { events "passed", "skipped", "failed" } }
//----
sourceSets { main { java { srcDirs = ['src/main/java', 'gen'] } } }
//----
android { compileSdk 34
  defaultConfig { minSdk 21; targetSdkVersion 34 } }
//----
android { buildTypes { release { minifyEnabled false; proguardFiles getDefaultProguardFile('proguard-android.txt'), 'proguard-rules.pro' } } }
//----
buildscript { repositories { google() }
  dependencies { classpath "com.android.tools.build:gradle:8.1.0" } }
//----
allprojects { repositories { mavenCentral() } }
//----
subprojects { project -> apply plugin: 'java' }
//----
include ':app', ':lib'
//----
include 'a',
        'b'
//----
rootProject.name = 'x'
//----
if (project.hasProperty('release')) { version = '1.0' } else { version = '1.0-SNAPSHOT' }
//----
def x = project.findProperty('a') ?: 'b'
//----
def y = a?.b?.c
//----
def list = files.collect { it.name }.findAll { it =~ /\.jar$/ }
//----
def m = [a: 1, 'b': 2, (c): 3]
//----
def s = '''multi
line'''
//----
def s = """multi ${a} line
 $b"""
//----
def re = ~/pattern\d+/
//----
def f = { a, b -> a + b }
//----
def f = { -> println 'x' }
//----
configurations.all { resolutionStrategy.eachDependency { details -> if (details.requested.group == 'x') { details.useVersion '1' } } }
//----
import org.gradle.api.tasks.Copy
import static java.lang.Math.max
//----
class Foo extends DefaultTask { @TaskAction void run() { println 'x' } }
//----
@Field def a = 1
//----
publishing { publications { mavenJava(MavenPublication) { from components.java } } }
//----
jar { manifest { attributes 'Main-Class': 'x.Y', 'Version': version } }
//----
println "a" + 'b'
//----
exec { commandLine 'git', 'rev-parse', 'HEAD'; standardOutput = out }
//----
dependencies { compile fileTree(dir: 'libs', include: ['*.jar']) }
//----
afterEvaluate { project -> project.tasks.findByName('x')?.dependsOn 'y' }
//----
tasks.each { t -> if (t.name ==~ /.*Test/) t.enabled = false }
//----
switch (os) { case 'linux': a = 1; break; default: a = 2 }
//----
try { foo() } catch (Exception e) { throw new GradleException("x", e) } finally { bar() }
//----
for (int i = 0; i < 3; i++) { println i }
//----
for (x in list) println x
//----
def a = b as String
//----
def a = (String) b
//----
def a = x instanceof Foo ? 1 : 2
//----
assert a == b : "msg"
//----
def a = new File(rootDir, 'x').text.readLines()*.trim()
//----
foo.bar baz, qux: 1
//----
foo bar { baz }
//----
a.b.c = d.e.f
//----
def x = 1_000L + 0x1F + 1.5e3f
//----
def x = [*a, *: m]
//----
label: for (;;) { break label }
//----
String foo() { return 'x' }
//----
def foo(String a, int b = 1, Closure c) { c() }
//----
enum E { A, B }
//----
interface I { void m() }
//----
@Grab('x:y:1') import x.Y
//----
pluginManagement { repositories { gradlePluginPortal() } }
//----
dependencyResolutionManagement { versionCatalogs { libs { from(files("../libs.versions.toml")) } } }
