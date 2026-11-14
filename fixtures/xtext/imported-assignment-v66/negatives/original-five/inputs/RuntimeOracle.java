// Independently authored construction oracle; never used in production.
package intersect.oracle;
import static intersect.oracle.MetamodelOracle.*;
import java.io.*;
import java.nio.charset.StandardCharsets;
import java.util.*;
import org.eclipse.emf.common.util.URI;
import org.eclipse.emf.ecore.*;
import org.eclipse.xtext.*;
import org.eclipse.xtext.parser.*;
import org.eclipse.xtext.resource.XtextResourceSet;

public final class RuntimeOracle {
  static Object wire(Object value,IdentityHashMap<EObject,Integer> ids) {
    if(value instanceof EObject) {
      var object=(EObject)value;
      if(ids.containsKey(object))return fields("object",ids.get(object),"class",identity(object.eClass()));
      return fields("external",org.eclipse.emf.ecore.util.EcoreUtil.getURI(object).toString(),"class",identity(object.eClass()));
    }
    if(value instanceof Iterable<?>) {
      List<Object> result=new ArrayList<>();for(Object item:(Iterable<?>)value)result.add(wire(item,ids));return result;
    }
    return value;
  }
  static Object model(EObject object) {
    if(object==null)return null;
    List<Object> result=new ArrayList<>();List<EObject> objects=new ArrayList<>();
    objects.add(object);var all=object.eAllContents();while(all.hasNext())objects.add(all.next());
    IdentityHashMap<EObject,Integer> ids=new IdentityHashMap<>();for(int i=0;i<objects.size();i++)ids.put(objects.get(i),i);
    for(EObject current:objects) {
      Map<String,Object> values=new LinkedHashMap<>();
      for(var feature:current.eClass().getEAllStructuralFeatures())values.put(feature.getName(),wire(current.eGet(feature),ids));
      result.add(fields("class",identity(current.eClass()),"values",values));
    }
    return result;
  }
  public static void main(String[] args)throws Exception {
    var setup=(org.eclipse.xtext.ISetup)Class.forName(args[0]+"StandaloneSetup").getConstructor().newInstance();
    var injector=setup.createInjectorAndDoEMFRegistration();String mode=args[1],text=args[2];
    Map<String,Object> result=fields("grammar",args[0],"entry",mode,"text",text);
    org.eclipse.emf.ecore.resource.Resource resource=null;
    try {
      EObject root;List<Object> errors=new ArrayList<>();
      if(mode.equals("resource")) {
        var resources=injector.getInstance(XtextResourceSet.class);
        resource=resources.createResource(URI.createURI("memory:/model.fragmenttest"));
        resource.load(new ByteArrayInputStream(text.getBytes(StandardCharsets.UTF_8)),null);
        for(var error:resource.getErrors())errors.add(fields("message",error.getMessage(),"line",error.getLine(),"column",error.getColumn()));
        root=resource.getContents().isEmpty()?null:resource.getContents().get(0);
      } else {
        var grammar=injector.getInstance(IGrammarAccess.class).getGrammar();
        var rule=GrammarUtil.findRuleForName(grammar,mode);
        var parsed=injector.getInstance(IParser.class).parse((ParserRule)rule,new StringReader(text));
        for(var error:parsed.getSyntaxErrors())errors.add(fields("message",error.getSyntaxErrorMessage().getMessage(),"start",error.getOffset(),"end",error.getEndOffset()));
        root=parsed.getRootASTElement();
      }
      result.put("accepted",root!=null&&errors.isEmpty());result.put("errors",errors);result.put("model",model(root));
      result.put("exception",null);result.put("resource_contents",resource==null?null:resource.getContents().size());
    } catch(Throwable error) {
      error.printStackTrace(System.err);result.put("accepted",false);
      result.put("resource_contents",resource==null?null:resource.getContents().size());
      result.put("model",resource==null||resource.getContents().isEmpty()?null:model(resource.getContents().get(0)));
      List<Object> chain=new ArrayList<>();
      for(Throwable current=error;current!=null;current=current.getCause())chain.add(fields("class",current.getClass().getName(),"message",current.getMessage()));
      result.put("exception",chain);
    }
    System.out.println(json(result));
  }
}
