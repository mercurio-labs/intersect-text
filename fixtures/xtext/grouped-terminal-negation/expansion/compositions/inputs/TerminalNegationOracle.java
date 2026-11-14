// Verification-only adapter to exact pinned Xtext translation and ANTLR runtime.
package intersect.oracle;
import java.io.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.util.*;
import org.eclipse.emf.common.util.URI;
import org.eclipse.emf.ecore.*;
import org.eclipse.emf.ecore.resource.Resource;
import org.eclipse.emf.ecore.util.EcoreUtil;
import org.eclipse.xtext.*;
import org.eclipse.xtext.resource.*;
import org.eclipse.xtext.util.CancelIndicator;
import org.eclipse.xtext.validation.*;
import org.eclipse.xtext.xtext.RuleNames;
import org.eclipse.xtext.xtext.generator.parser.antlr.TerminalRuleToLexerBody;
import org.eclipse.xtext.xtext.generator.parser.antlr.AntlrGrammarGenUtil;
import org.antlr.runtime.*;
public final class TerminalNegationOracle {
 static Map<String,Object> fields(Object... pairs){Map<String,Object> m=new LinkedHashMap<>();for(int i=0;i<pairs.length;i+=2)m.put((String)pairs[i],pairs[i+1]);return m;}
 static String quote(String s){if(s==null)return "null";StringBuilder b=new StringBuilder("\"");for(char c:s.toCharArray())switch(c){case '\\':b.append("\\\\");break;case '"':b.append("\\\"");break;case '\n':b.append("\\n");break;case '\r':b.append("\\r");break;case '\t':b.append("\\t");break;default:if(c<32||Character.isSurrogate(c))b.append(String.format("\\u%04x",(int)c));else b.append(c);}return b.append('"').toString();}
 static String json(Object v){if(v==null)return "null";if(v instanceof String)return quote((String)v);if(v instanceof Number||v instanceof Boolean)return v.toString();StringJoiner j=new StringJoiner(",",v instanceof Map?"{":"[",v instanceof Map?"}":"]");if(v instanceof Map)((Map<?,?>)v).forEach((k,x)->j.add(quote((String)k)+":"+json(x)));else for(Object x:(Iterable<?>)v)j.add(json(x));return j.toString();}
 static Object failure(Throwable t){return fields("class",t.getClass().getName(),"message",t.getMessage());}
 static String owner(AbstractRule r){return r==null?null:GrammarUtil.getGrammar(r).getName()+"::"+r.getName();}
 static void metadata(Path root)throws Exception{
  var injector=new XtextStandaloneSetup().createInjectorAndDoEMFRegistration();var set=injector.getInstance(XtextResourceSet.class);var validator=injector.getInstance(IResourceValidator.class);
  var imported=set.getResource(URI.createFileURI(root.resolve("imported.ecore").toString()),true);var pkg=(EPackage)imported.getContents().get(0);set.getPackageRegistry().put(pkg.getNsURI(),pkg);
  Map<String,XtextResource> loaded=new LinkedHashMap<>();List<Object> syntax=new ArrayList<>();
  for(String name:Files.readAllLines(root.resolve("order.txt"),StandardCharsets.UTF_8)){
   var resource=(XtextResource)set.createResource(URI.createURI("memory:/"+name.replace('.','/')+".xtext"));resource.load(new ByteArrayInputStream(Files.readAllBytes(root.resolve(name+".xtext"))),null);loaded.put(name,resource);
   List<Object> errors=new ArrayList<>();for(var n:resource.getParseResult().getSyntaxErrors())errors.add(fields("start",n.getOffset(),"length",n.getLength(),"message",n.getSyntaxErrorMessage().getMessage()));syntax.add(fields("source",name,"errors",errors));
  }
  Object linkingFailure=null;try{EcoreUtil.resolveAll(set);}catch(Throwable t){linkingFailure=failure(t);}
  List<Object> linked=new ArrayList<>(),validations=new ArrayList<>();
  for(var item:loaded.entrySet()){
   List<Object> errors=new ArrayList<>(),warnings=new ArrayList<>(),calls=new ArrayList<>(),issues=new ArrayList<>();for(var d:item.getValue().getErrors())errors.add(fields("line",d.getLine(),"column",d.getColumn(),"message",d.getMessage()));for(var d:item.getValue().getWarnings())warnings.add(fields("line",d.getLine(),"column",d.getColumn(),"message",d.getMessage()));
   var contents=item.getValue().getAllContents();while(contents.hasNext()){var value=contents.next();if(value instanceof RuleCall){var c=(RuleCall)value;calls.add(fields("target",c.getRule()==null||c.getRule().eIsProxy()?null:owner(c.getRule()),"proxy",c.getRule()!=null&&c.getRule().eIsProxy(),"explicit",c.isExplicitlyCalled()));}}
   linked.add(fields("source",item.getKey(),"errors",errors,"warnings",warnings,"calls",calls));Object validationFailure=null;
   try{for(var issue:validator.validate(item.getValue(),CheckMode.ALL,CancelIndicator.NullImpl))issues.add(fields("severity",issue.getSeverity().name(),"code",issue.getCode(),"start",issue.getOffset(),"length",issue.getLength(),"message",issue.getMessage()));}catch(Throwable t){validationFailure=failure(t);}issues.sort(Comparator.comparing(TerminalNegationOracle::json));validations.add(fields("source",item.getKey(),"issues",issues,"failure",validationFailure));
  }
  List<Object> rules=new ArrayList<>();Object translationFailure=null;
  try{String entry=Files.readString(root.resolve("entry.txt"),StandardCharsets.UTF_8).trim();Grammar grammar=(Grammar)loaded.get(entry).getContents().get(0);new RuleNames(grammar,true);for(var rule:GrammarUtil.allTerminalRules(grammar))rules.add(fields("source_rule",owner(rule),"name",AntlrGrammarGenUtil.getRuleName(rule),"fragment",rule.isFragment(),"body",TerminalRuleToLexerBody.toLexerBody(rule)));}catch(Throwable t){translationFailure=failure(t);}
  System.out.println(json(fields("syntax",syntax,"linking",linked,"linking_failure",linkingFailure,"validation",validations,"translated_rules",rules,"translation_failure",translationFailure)));
 }
 static void lex(String[] args)throws Exception{
  Class<?> cls=Class.forName(args[1]);Map<Integer,String> names=new TreeMap<>();for(var field:cls.getFields())if(field.getType()==int.class&&java.lang.reflect.Modifier.isStatic(field.getModifiers()))names.put(field.getInt(null),field.getName());
  for(int i=2;i<args.length;i++){
   String text=new String(Base64.getDecoder().decode(args[i]),StandardCharsets.UTF_8);Lexer lexer=(Lexer)cls.getConstructor(CharStream.class).newInstance(new ANTLRStringStream(text));List<Object> tokens=new ArrayList<>();Object failed=null;
   try{for(int n=0;n<text.length()+4;n++){Token token=lexer.nextToken();CommonToken c=(CommonToken)token;tokens.add(fields("type",token.getType(),"name",token.getType()==Token.EOF?"EOF":names.get(token.getType()),"text",token.getText(),"utf16_start",c.getStartIndex(),"utf16_stop",c.getStopIndex(),"channel",token.getChannel()));if(token.getType()==Token.EOF)break;}}catch(Throwable t){failed=failure(t);}
   int errors=cls.getField("errors").getInt(lexer);boolean complete=tokens.size()==2&&errors==0&&failed==null; if(complete){@SuppressWarnings("unchecked")Map<String,Object> first=(Map<String,Object>)tokens.get(0);complete=Objects.equals(first.get("name"),"RULE_VALUE")&&text.equals(first.get("text"));}
   System.out.println(json(fields("text",text,"tokens",tokens,"lexer_errors",errors,"complete_single_value",complete,"failure",failed)));
  }
 }
 public static void main(String[] args)throws Exception{if(args[0].equals("metadata"))metadata(Path.of(args[1]).toAbsolutePath());else lex(args);}
}
