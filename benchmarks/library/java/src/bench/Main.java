package bench;

import java.nio.file.Files;
import java.nio.file.Path;
import java.io.ByteArrayInputStream;
import java.util.Map;
import org.eclipse.emf.common.util.URI;
import org.eclipse.xtext.resource.XtextResourceSet;
import example.LibraryStandaloneSetup;

/** Handwritten measurement host; all parsing and EMF classes are generated. */
public final class Main {
    public static void main(String[] args) throws Exception {
        var injector = new LibraryStandaloneSetup().createInjectorAndDoEMFRegistration();
        var resources = injector.getInstance(XtextResourceSet.class);
        var resource = resources.createResource(URI.createURI("benchmark.library"));
        byte[] input = Files.readAllBytes(Path.of(args[0]));
        int expected = Integer.parseInt(args[1]);
        System.out.println("READY");
        System.in.read();
        long start = System.nanoTime();
        resource.load(new ByteArrayInputStream(input), Map.of());
        double elapsed = (System.nanoTime() - start) / 1e6;
        int count = 0;
        boolean verified = resource.getErrors().isEmpty();
        var contents = resource.getAllContents();
        while (contents.hasNext()) {
            var element = contents.next();
            String name = count == 0 ? "Scale" : "m" + count;
            String type = count == 0 ? "Library" : "Member";
            verified &= type.equals(element.eClass().getName());
            verified &= name.equals(element.eGet(element.eClass().getEStructuralFeature("name")));
            count++;
        }
        verified &= count == expected;
        System.out.println("MEASURED");
        System.in.read();
        System.out.printf(java.util.Locale.ROOT,
            "{\"engine\":\"xtext\",\"stage\":\"resource_load\",\"status\":\"%s\",\"elapsed_ms\":%.6f,\"elements\":%d,\"verified\":%s,\"diagnostic_count\":%d}%n",
            verified ? "ok" : "rejected", elapsed, count, verified, resource.getErrors().size());
        java.lang.ref.Reference.reachabilityFence(resource);
        java.lang.ref.Reference.reachabilityFence(input);
    }
}
