// Independent generated-entry return oracle; never a production dependency.
package intersect.oracle;
import static intersect.oracle.MetamodelOracle.*;
import java.lang.reflect.*;
import java.util.*;
import org.antlr.runtime.*;
import org.eclipse.emf.ecore.*;
import org.eclipse.xtext.parser.IParser;
import org.eclipse.xtext.parser.antlr.*;
import org.eclipse.xtext.nodemodel.impl.NodeModelBuilder;

public final class ScalarEntryOracle {
  static Object call(Object target,String name,Class<?>[] types,Object... args)throws Exception {
    for(Class<?> c=target.getClass();c!=null;c=c.getSuperclass()) {
      try {var m=c.getDeclaredMethod(name,types);m.setAccessible(true);return m.invoke(target,args);}
      catch(NoSuchMethodException absent) {}
    }
    throw new NoSuchMethodException(name);
  }
  public static void main(String[] args)throws Exception {
    for(String name:args[0].split(",")) {
      EPackage p=(EPackage)Class.forName(name).getField("eINSTANCE").get(null);
      EPackage.Registry.INSTANCE.put(p.getNsURI(),p);
    }
    var setup=(org.eclipse.xtext.ISetup)Class.forName(args[1]+"StandaloneSetup").getConstructor().newInstance();
    var injector=setup.createInjectorAndDoEMFRegistration();
    var result=fields("grammar",args[1],"entry",args[2],"text",args[3],"mode","direct_generated_entry");
    try {
      var frontend=(AbstractAntlrParser)injector.getInstance(IParser.class);
      var lexer=(TokenSource)call(frontend,"createLexer",new Class<?>[]{CharStream.class},new ANTLRStringStream(args[3]));
      var tokens=(XtextTokenStream)call(frontend,"createTokenStream",new Class<?>[]{TokenSource.class},lexer);
      tokens.initCurrentLookAhead(0);
      call(frontend,"setInitialHiddenTokens",new Class<?>[]{XtextTokenStream.class},tokens);
      var parser=(AbstractInternalAntlrParser)call(frontend,"createParser",new Class<?>[]{XtextTokenStream.class},tokens);
      parser.setTokenTypeMap(frontend.getTokenDefProvider().getTokenDefMap());
      parser.setSyntaxErrorProvider(frontend.getSyntaxErrorProvider());
      var builder=(NodeModelBuilder)call(frontend,"createNodeModelBuilder",new Class<?>[]{});
      parser.setNodeModelBuilder(builder);
      parser.setSemanticModelBuilder(frontend.getElementFactory());
      var helper=frontend.getUnorderedGroupHelper().get();
      parser.setUnorderedGroupHelper(helper);helper.initializeWith(parser);
      var current=AbstractInternalAntlrParser.class.getDeclaredField("currentNode");current.setAccessible(true);
      String content=tokens.toString();current.set(parser,builder.newRootNode(content==null?"":content));
      List<Object> inventory=new ArrayList<>();
      var methods=Arrays.stream(parser.getClass().getMethods()).filter(m->m.getName().startsWith("entryRule")&&m.getParameterCount()==0).sorted(Comparator.comparing(Method::getName)).toList();
      for(var method:methods)inventory.add(fields("method",method.getName(),"returns",method.getReturnType().getName()));
      result.put("entry_methods",inventory);
      var method=parser.getClass().getMethod("entryRule"+args[2]);
      result.put("declared_return_type",method.getReturnType().getName());
      Object value=method.invoke(parser);
      result.put("value_kind",value==null?"null":value instanceof String?"string":value instanceof EObject?"model":"unsupported");
      result.put("value",value instanceof EObject?RuntimeOracle.model((EObject)value):value);
      call(parser,"appendSkippedTokens",new Class<?>[]{});
      call(parser,"appendTrailingHiddenTokens",new Class<?>[]{});
      var root=((org.eclipse.xtext.nodemodel.ICompositeNode)current.get(parser)).getRootNode();
      List<Object> errors=new ArrayList<>();
      for(var node:root.getAsTreeIterable()) {
        if(node.getSyntaxErrorMessage()!=null)errors.add(fields("message",node.getSyntaxErrorMessage().getMessage(),"start",node.getOffset(),"end",node.getEndOffset()));
      }
      result.put("errors",errors);
      result.put("antlr_syntax_error_counter",parser.getNumberOfSyntaxErrors());result.put("exception",null);
    } catch(Throwable error) {
      error.printStackTrace(System.err);List<Object> chain=new ArrayList<>();
      for(Throwable c=error;c!=null;c=c.getCause())chain.add(fields("class",c.getClass().getName(),"message",c.getMessage()));
      result.put("exception",chain);
    }
    System.out.println(json(result));
  }
}
