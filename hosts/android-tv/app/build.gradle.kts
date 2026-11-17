import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

// Hearth TV's own version (the owner picks it): hosts/android-tv/VERSION. 1.0 -> 1000000, 1.2.3 -> 1002003.
val tvVersion: String = rootProject.layout.projectDirectory.file("VERSION").asFile.readText().trim()
val tvVersionCode: Int = tvVersion.split(".").map { it.toInt() }.let { p -> p.getOrElse(0) { 0 } * 1_000_000 + p.getOrElse(1) { 0 } * 1_000 + p.getOrElse(2) { 0 } }

// Release signing: the repository's key when CI has one (the same secrets as Hearth for Android), otherwise the
// debug key, so the APK always installs. See .github/workflows/android-tv.yml.
val releaseKeystore: String? = System.getenv("HEARTH_KEYSTORE")?.takeIf { it.isNotBlank() && file(it).exists() }

android {
    namespace = "dev.hearthos.tv"
    compileSdk = 35

    defaultConfig {
        applicationId = "dev.hearthos.tv"
        // Android TV 8.0 and newer (every Google TV set is newer than that).
        minSdk = 26
        targetSdk = 35
        versionCode = tvVersionCode
        versionName = tvVersion
    }

    signingConfigs {
        if (releaseKeystore != null) {
            create("release") {
                storeFile = file(releaseKeystore)
                storePassword = System.getenv("HEARTH_KEYSTORE_PASSWORD")
                keyAlias = System.getenv("HEARTH_KEY_ALIAS")
                keyPassword = System.getenv("HEARTH_KEY_PASSWORD")
            }
        }
    }

    buildTypes {
        getByName("debug") {
            applicationIdSuffix = ".debug"
            versionNameSuffix = "-debug"
        }
        getByName("release") {
            isMinifyEnabled = false
            signingConfig = signingConfigs.findByName("release") ?: signingConfigs.getByName("debug")
        }
    }

    buildFeatures { buildConfig = true }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    lint {
        abortOnError = false
        checkReleaseBuilds = false
    }

    // Hearth Link for Android (lib/<abi>/libhearthlink.so, built by CI from hosts/hearth-link) runs as its own
    // program, so it must be unpacked into the app's native library folder at install.
    packaging { jniLibs { useLegacyPackaging = true } }
}

kotlin { compilerOptions { jvmTarget.set(JvmTarget.JVM_17) } }

// Hearth OS's looks (palettes.json: every look, its sub-themes, the Black set and the accents; Gold when it lands
// there) and their wallpapers, copied into the APK when it's built so Hearth TV always has the same looks as Hearth OS.
abstract class CopyHearthLooks : DefaultTask() {
    @get:InputFiles
    @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val sources: ConfigurableFileCollection

    @get:Internal
    abstract val repo: DirectoryProperty

    @get:OutputDirectory
    abstract val outputDir: DirectoryProperty

    @TaskAction
    fun copy() {
        val out = outputDir.get().asFile.resolve("tv/theme")
        outputDir.get().asFile.deleteRecursively()
        out.mkdirs()
        val r = repo.get().asFile
        val pal = r.resolve("hosts/linux/desktop/theme/palettes.json")
        if (!pal.exists()) throw GradleException("palettes.json is missing: Hearth TV's looks come from it")
        pal.copyTo(out.resolve("palettes.json"), overwrite = true)
        r.resolve("hosts/linux/desktop/shell/images/wallpapers").walkTopDown()
            .filter { it.isFile && it.extension == "jpg" && "thumbs" !in it.path }
            .forEach { f -> f.copyTo(out.resolve("wallpapers/" + f.relativeTo(r.resolve("hosts/linux/desktop/shell/images/wallpapers")).path), overwrite = true) }
        r.resolve("os/branding/wallpapers").listFiles { f -> f.extension == "jpg" }?.forEach { f -> f.copyTo(out.resolve("backgrounds/" + f.name), overwrite = true) }
    }
}

val copyHearthLooks = tasks.register<CopyHearthLooks>("copyHearthLooks") {
    val r = rootProject.layout.projectDirectory.dir("../..")
    repo.set(r)
    sources.from(r.file("hosts/linux/desktop/theme/palettes.json"), r.dir("hosts/linux/desktop/shell/images/wallpapers"), r.dir("os/branding/wallpapers"))
}

androidComponents {
    onVariants { variant ->
        variant.sources.assets?.addGeneratedSourceDirectory(copyHearthLooks, CopyHearthLooks::outputDir)
    }
}

dependencies {
    implementation("androidx.core:core-ktx:1.15.0")
    implementation("androidx.webkit:webkit:1.12.1")
    // TV Cleanup: Hearth TV talks to the TV's own adb (127.0.0.1:5555) with its own key (Apache 2.0).
    implementation("dev.mobile:dadb:1.2.10")
    // The QR code on Theater's Add a link (Apache 2.0).
    implementation("com.google.zxing:core:3.5.3")

    testImplementation("junit:junit:4.13.2")
    testImplementation("org.json:json:20240303")
}
