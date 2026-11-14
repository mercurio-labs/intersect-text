package audit.parser.antlr.internal;

import org.eclipse.xtext.*;
import org.eclipse.xtext.parser.*;
import org.eclipse.xtext.parser.impl.*;
import org.eclipse.emf.ecore.util.EcoreUtil;
import org.eclipse.emf.ecore.EObject;
import org.eclipse.xtext.parser.antlr.AbstractInternalAntlrParser;
import org.eclipse.xtext.parser.antlr.XtextTokenStream;
import org.eclipse.xtext.parser.antlr.XtextTokenStream.HiddenTokens;
import org.eclipse.xtext.parser.antlr.AntlrDatatypeRuleToken;
import audit.services.FirstRuleDatatypeLibraryChildGrammarAccess;



import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

@SuppressWarnings("all")
public class InternalFirstRuleDatatypeLibraryChildParser extends AbstractInternalAntlrParser {
    public static final String[] tokenNames = new String[] {
        "<invalid>", "<EOR>", "<DOWN>", "<UP>", "'a'"
    };
    public static final int T__4=4;
    public static final int EOF=-1;

    // delegates
    // delegators


        public InternalFirstRuleDatatypeLibraryChildParser(TokenStream input) {
            this(input, new RecognizerSharedState());
        }
        public InternalFirstRuleDatatypeLibraryChildParser(TokenStream input, RecognizerSharedState state) {
            super(input, state);
             
        }
        

    public String[] getTokenNames() { return InternalFirstRuleDatatypeLibraryChildParser.tokenNames; }
    public String getGrammarFileName() { return "InternalFirstRuleDatatypeLibraryChild.g"; }



     	private FirstRuleDatatypeLibraryChildGrammarAccess grammarAccess;

        public InternalFirstRuleDatatypeLibraryChildParser(TokenStream input, FirstRuleDatatypeLibraryChildGrammarAccess grammarAccess) {
            this(input);
            this.grammarAccess = grammarAccess;
            registerRules(grammarAccess.getGrammar());
        }

        @Override
        protected String getFirstRuleName() {
        	return "Model";
       	}

       	@Override
       	protected FirstRuleDatatypeLibraryChildGrammarAccess getGrammarAccess() {
       		return grammarAccess;
       	}




    // $ANTLR start "entryRuleModel"
    // InternalFirstRuleDatatypeLibraryChild.g:64:1: entryRuleModel returns [EObject current=null] : iv_ruleModel= ruleModel EOF ;
    public final EObject entryRuleModel() throws RecognitionException {
        EObject current = null;

        EObject iv_ruleModel = null;


        try {
            // InternalFirstRuleDatatypeLibraryChild.g:64:46: (iv_ruleModel= ruleModel EOF )
            // InternalFirstRuleDatatypeLibraryChild.g:65:2: iv_ruleModel= ruleModel EOF
            {
             newCompositeNode(grammarAccess.getModelRule()); 
            pushFollow(FOLLOW_1);
            iv_ruleModel=ruleModel();

            state._fsp--;

             current =iv_ruleModel; 
            match(input,EOF,FOLLOW_2); 

            }

        }

            catch (RecognitionException re) {
                recover(input,re);
                appendSkippedTokens();
            }
        finally {
        }
        return current;
    }
    // $ANTLR end "entryRuleModel"


    // $ANTLR start "ruleModel"
    // InternalFirstRuleDatatypeLibraryChild.g:71:1: ruleModel returns [EObject current=null] : ( (lv_name_0_0= ruleText ) ) ;
    public final EObject ruleModel() throws RecognitionException {
        EObject current = null;

        AntlrDatatypeRuleToken lv_name_0_0 = null;



        	enterRule();

        try {
            // InternalFirstRuleDatatypeLibraryChild.g:77:2: ( ( (lv_name_0_0= ruleText ) ) )
            // InternalFirstRuleDatatypeLibraryChild.g:78:2: ( (lv_name_0_0= ruleText ) )
            {
            // InternalFirstRuleDatatypeLibraryChild.g:78:2: ( (lv_name_0_0= ruleText ) )
            // InternalFirstRuleDatatypeLibraryChild.g:79:3: (lv_name_0_0= ruleText )
            {
            // InternalFirstRuleDatatypeLibraryChild.g:79:3: (lv_name_0_0= ruleText )
            // InternalFirstRuleDatatypeLibraryChild.g:80:4: lv_name_0_0= ruleText
            {

            				newCompositeNode(grammarAccess.getModelAccess().getNameTextParserRuleCall_0());
            			
            pushFollow(FOLLOW_2);
            lv_name_0_0=ruleText();

            state._fsp--;


            				if (current==null) {
            					current = createModelElementForParent(grammarAccess.getModelRule());
            				}
            				set(
            					current,
            					"name",
            					lv_name_0_0,
            					"audit.FirstRuleDatatypeLibraryBase.Text");
            				afterParserOrEnumRuleCall();
            			

            }


            }


            }


            	leaveRule();

        }

            catch (RecognitionException re) {
                recover(input,re);
                appendSkippedTokens();
            }
        finally {
        }
        return current;
    }
    // $ANTLR end "ruleModel"


    // $ANTLR start "entryRuleText"
    // InternalFirstRuleDatatypeLibraryChild.g:100:1: entryRuleText returns [String current=null] : iv_ruleText= ruleText EOF ;
    public final String entryRuleText() throws RecognitionException {
        String current = null;

        AntlrDatatypeRuleToken iv_ruleText = null;


        try {
            // InternalFirstRuleDatatypeLibraryChild.g:100:44: (iv_ruleText= ruleText EOF )
            // InternalFirstRuleDatatypeLibraryChild.g:101:2: iv_ruleText= ruleText EOF
            {
             newCompositeNode(grammarAccess.getTextRule()); 
            pushFollow(FOLLOW_1);
            iv_ruleText=ruleText();

            state._fsp--;

             current =iv_ruleText.getText(); 
            match(input,EOF,FOLLOW_2); 

            }

        }

            catch (RecognitionException re) {
                recover(input,re);
                appendSkippedTokens();
            }
        finally {
        }
        return current;
    }
    // $ANTLR end "entryRuleText"


    // $ANTLR start "ruleText"
    // InternalFirstRuleDatatypeLibraryChild.g:107:1: ruleText returns [AntlrDatatypeRuleToken current=new AntlrDatatypeRuleToken()] : kw= 'a' ;
    public final AntlrDatatypeRuleToken ruleText() throws RecognitionException {
        AntlrDatatypeRuleToken current = new AntlrDatatypeRuleToken();

        Token kw=null;


        	enterRule();

        try {
            // InternalFirstRuleDatatypeLibraryChild.g:113:2: (kw= 'a' )
            // InternalFirstRuleDatatypeLibraryChild.g:114:2: kw= 'a'
            {
            kw=(Token)match(input,4,FOLLOW_2); 

            		current.merge(kw);
            		newLeafNode(kw, grammarAccess.getTextAccess().getAKeyword());
            	

            }


            	leaveRule();

        }

            catch (RecognitionException re) {
                recover(input,re);
                appendSkippedTokens();
            }
        finally {
        }
        return current;
    }
    // $ANTLR end "ruleText"

    // Delegated rules


 

    public static final BitSet FOLLOW_1 = new BitSet(new long[]{0x0000000000000000L});
    public static final BitSet FOLLOW_2 = new BitSet(new long[]{0x0000000000000002L});

}