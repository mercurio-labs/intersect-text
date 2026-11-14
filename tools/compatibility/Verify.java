// Handwritten independent EMF oracle. Verification host only; never production.
import java.nio.file.*;
import java.util.*;
import org.eclipse.emf.common.util.URI;
import org.eclipse.emf.ecore.*;
import org.eclipse.emf.ecore.resource.*;
import org.eclipse.emf.ecore.resource.impl.ResourceSetImpl;
import org.eclipse.emf.ecore.util.*;
import org.eclipse.emf.ecore.xmi.XMLResource;
import org.eclipse.emf.ecore.xmi.impl.*;

public class Verify {
  static String key(EObject object) {
    EClass type=object.eClass();
    Object name=object.eGet(type.getEStructuralFeature("name"));
    return type.getEPackage().getNsURI()+"#//"+type.getName()+":"+name;
  }
  static Object value(Object value) {
    if (value instanceof EObject o) return key(o);
    if (value instanceof Collection<?> list) return list.stream().map(Verify::value).toList();
    return value;
  }
  static String json(Object v) {
    if(v==null) return "null";
    if(v instanceof Map<?,?> m) return "{"+String.join(",",m.entrySet().stream().map(e->json(e.getKey().toString())+":"+json(e.getValue())).toList())+"}";
    if(v instanceof Collection<?> c) return "["+String.join(",",c.stream().map(Verify::json).toList())+"]";
    if(v instanceof Boolean || v instanceof Number) return v.toString();
    String s=v.toString();StringBuilder b=new StringBuilder("\"");
    for(int i=0;i<s.length();i++){char c=s.charAt(i);if(c=='"'||c=='\\')b.append('\\').append(c);else if(c<32)b.append(String.format("\\u%04x",(int)c));else b.append(c);}
    return b.append('"').toString();
  }
  public static void main(String[] args) throws Exception {
    Path input=Path.of(args[0]).toAbsolutePath(),output=Path.of(args[1]).toAbsolutePath();
    int checked=0;
    for(int seed=0;seed<8;seed++) for(String origin:List.of("dynamic","text")) {
      Path folder=input.resolve("seed-"+seed).resolve(origin),dest=output.resolve("seed-"+seed).resolve(origin);
      ResourceSet rs=new ResourceSetImpl();
      rs.getResourceFactoryRegistry().getExtensionToFactoryMap().put("ecore",new EcoreResourceFactoryImpl());
      rs.getResourceFactoryRegistry().getExtensionToFactoryMap().put("xmi",new XMIResourceFactoryImpl());
      Resource definitions=rs.getResource(URI.createFileURI(input.resolve("graph.ecore").toString()),true);
      EPackage pack=(EPackage)definitions.getContents().get(0);rs.getPackageRegistry().put(pack.getNsURI(),pack);
      List<Resource> resources=new ArrayList<>();
      try(var stream=Files.list(folder)){for(Path file:stream.filter(p->p.toString().endsWith(".xmi")).sorted().toList())resources.add(rs.getResource(URI.createFileURI(file.toString()),true));}
      EcoreUtil.resolveAll(rs);
      Map<String,Object> graph=new TreeMap<>();
      for(Resource r:resources) {
        if(!r.getErrors().isEmpty())throw new IllegalStateException(r.getErrors().toString());
        var objects=r.getAllContents();
        while(objects.hasNext()) {
          EObject object=objects.next();if(object.eIsProxy())throw new IllegalStateException("Unresolved proxy");
          var diagnosis=Diagnostician.INSTANCE.validate(object);
          if(diagnosis.getSeverity()>=4)throw new IllegalStateException(diagnosis.toString());
          Map<String,Object> fields=new TreeMap<>();
          for(EStructuralFeature f:object.eClass().getEAllStructuralFeatures())if(!f.isDerived())fields.put(f.getName(),value(object.eGet(f)));
          if(graph.put(key(object),fields)!=null)throw new IllegalStateException("Duplicate correspondence key");
        }
      }
      Files.createDirectories(dest);Files.writeString(dest.resolve("normalized.json"),json(graph));
      for(Resource r:resources){((XMLResource)r).getEObjectToExtensionMap().clear();r.setURI(URI.createFileURI(dest.resolve(r.getURI().lastSegment()).toString()));}
      for(Resource r:resources)r.save(Map.of(XMLResource.OPTION_ENCODING,"UTF-8"));
      checked++;System.out.println("Verified EMF load/validate/save: seed "+seed+" "+origin+", "+graph.size()+" objects");
    }
    System.out.println("Completed "+checked+" independent resource-set controls.");
  }
}
