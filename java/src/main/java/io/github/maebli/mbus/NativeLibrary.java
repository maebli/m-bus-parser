package io.github.maebli.mbus;

import java.io.IOException;
import java.io.InputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Locale;

final class NativeLibrary {
    private NativeLibrary() {}

    private static final class Bundled {
        private static final Path PATH = extract();
    }

    static Path path() {
        String configured = System.getProperty("mbus.parser.library");
        return configured == null ? Bundled.PATH : Path.of(configured).toAbsolutePath();
    }

    private static Path extract() {
        String osName = System.getProperty("os.name").toLowerCase(Locale.ROOT);
        String os;
        if (osName.startsWith("windows")) {
            os = "windows";
        } else if (osName.startsWith("mac")) {
            os = "macos";
        } else if (osName.equals("linux")) {
            os = "linux";
        } else {
            throw new UnsupportedOperationException("Unsupported operating system: " + osName);
        }
        String architecture = switch (System.getProperty("os.arch").toLowerCase(Locale.ROOT)) {
            case "amd64", "x86_64" -> "x86_64";
            case "aarch64", "arm64" -> "aarch64";
            default -> throw new UnsupportedOperationException("Unsupported CPU architecture");
        };
        String name = System.mapLibraryName("mbus_parser");
        String resource = "/native/" + os + "-" + architecture + "/" + name;
        try (InputStream source = NativeLibrary.class.getResourceAsStream(resource)) {
            if (source == null) {
                throw new UnsatisfiedLinkError("Native library is missing: " + resource
                        + ". Set mbus.parser.library to a compatible library path.");
            }
            Path directory = Files.createTempDirectory("mbus-parser-");
            directory.toFile().deleteOnExit();
            Path library = directory.resolve(name);
            Files.copy(source, library);
            library.toFile().deleteOnExit();
            return library;
        } catch (IOException error) {
            throw new IllegalStateException("Cannot extract the bundled M-Bus parser", error);
        }
    }
}
