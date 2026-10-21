import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

// Hearth's version, from the Rust host's Cargo.toml (one version for every Hearth). 0.2.1 if it can't be read.
val hearthVersion: String = run {
    val cargo = rootProject.layout.projectDirectory.file("../hearth-rs/Cargo.toml").asFile
    val line = if (cargo.exists()) cargo.readLines().firstOrNull { it.trim().startsWith("version") } else null
    Regex("\"([0-9]+\\.[0-9]+\\.[0-9]+)[^\"]*\"").find(line ?: "")?.groupValues?.get(1) ?: "0.2.1"
}
// 0.2.1 -> 201, 1.4.12 -> 10412: always higher for a newer Hearth.
val hearthVersionCode: Int = hearthVersion.split(".").map { it.toInt() }.let { (a, b, c) -> a * 10000 + b * 100 + c }

// Release signing: a real key when CI has one (secrets → environment), otherwise the debug key, so the release
// APK always installs. See .github/workflows/android.yml.
val releaseKeystore: String? = System.getenv("HEARTH_KEYSTORE")?.takeIf { it.isNotBlank() && file(it).exists() }

android {
    namespace = "dev.hearthos.android"
    compileSdk = 35

    defaultConfig {
        applicationId = "dev.hearthos.android"
        minSdk = 26
        targetSdk = 35
        versionCode = hearthVersionCode
        versionName = hearthVersion
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

    testOptions { unitTests.isReturnDefaultValues = false }

    // Hearth Linux: PRoot is in jniLibs (lib/<abi>/libproot.so…, fetched by CI, see docs/ANDROID-LINUX.md) and must
    // be a real file in the app's native library folder: Android 10+ runs programs only from there, never from the
    // app's data. So the libraries are unpacked at install (extractNativeLibs=true) instead of staying in the APK.
    packaging { jniLibs { useLegacyPackaging = true } }
}

kotlin { compilerOptions { jvmTarget.set(JvmTarget.JVM_17) } }

// Hearth's page has one home: ui/ at the top of the repository. It's copied into the APK's assets (as ui/) when
// the app is built, never kept twice in git.
abstract class CopyHearthUi : DefaultTask() {
    @get:InputFiles
    @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val pageFiles: ConfigurableFileCollection

    @get:OutputDirectory
    abstract val outputDir: DirectoryProperty

    @TaskAction
    fun copy() {
        val out = outputDir.get().asFile.resolve("ui")
        outputDir.get().asFile.deleteRecursively()
        out.mkdirs()
        val files = pageFiles.files
        if (files.none { it.name == "index.html" }) throw GradleException("ui/index.html is missing: Hearth's page can't go into the app")
        files.forEach { it.copyTo(out.resolve(it.name), overwrite = true) }
    }
}

val copyHearthUi = tasks.register<CopyHearthUi>("copyHearthUi") {
    val ui = rootProject.layout.projectDirectory.dir("../../ui")
    pageFiles.from(ui.file("index.html"), ui.file("hearth-logo.svg"))
}

androidComponents {
    onVariants { variant ->
        variant.sources.assets?.addGeneratedSourceDirectory(copyHearthUi, CopyHearthUi::outputDir)
    }
}

dependencies {
    implementation("androidx.core:core-ktx:1.15.0")
    implementation("androidx.webkit:webkit:1.12.1")
    // Unpacking the Linux download (.tar.xz). XZ for Java: 0BSD, no other dependencies.
    implementation("org.tukaani:xz:1.10")

    testImplementation("junit:junit:4.13.2")
    // org.json is part of Android, but only as stubs in plain JVM unit tests: the real one for those.
    testImplementation("org.json:json:20240303")
}
