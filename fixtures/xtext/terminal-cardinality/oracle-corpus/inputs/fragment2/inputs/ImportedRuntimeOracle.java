package intersect.oracle;
import java.util.Arrays;
import org.eclipse.emf.ecore.EPackage;
public final class ImportedRuntimeOracle {
  public static void main(String[] args) throws Exception {
    for (String name : args[0].split(",")) {
      EPackage p = (EPackage)Class.forName(name).getField("eINSTANCE").get(null);
      EPackage.Registry.INSTANCE.put(p.getNsURI(), p);
    }
    RuntimeOracle.main(Arrays.copyOfRange(args,1,args.length));
  }
}
