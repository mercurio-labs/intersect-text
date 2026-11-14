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
import audit.services.ReturnMatrixEcoreUnaliasedImplicitGrammarAccess;



import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

@SuppressWarnings("all")
public class InternalReturnMatrixEcoreUnaliasedImplicitParser extends AbstractInternalAntlrParser {
    public static final String[] tokenNames = new String[] {
        "<invalid>", "<EOR>", "<DOWN>", "<UP>", "RULE_VALUE"
    };
    public static final int RULE_VALUE=4;
    public static final int EOF=-1;

    // delegates
    // delegators


        public InternalReturnMatrixEcoreUnaliasedImplicitParser(TokenStream input) {
            this(input, new RecognizerSharedState());
        }
        public InternalReturnMatrixEcoreUnaliasedImplicitParser(TokenStream input, RecognizerSharedState state) {
            super(input, state);
             
        }
        

    public String[] getTokenNames() { return InternalReturnMatrixEcoreUnaliasedImplicitParser.tokenNames; }
    public String getGrammarFileName() { return "InternalReturnMatrixEcoreUnaliasedImplicit.g"; }



     	private ReturnMatrixEcoreUnaliasedImplicitGrammarAccess grammarAccess;

        public InternalReturnMatrixEcoreUnaliasedImplicitParser(TokenStream input, ReturnMatrixEcoreUnaliasedImplicitGrammarAccess grammarAccess) {
            this(input);
            this.grammarAccess = grammarAccess;
            registerRules(grammarAccess.getGrammar());
        }

        @Override
        protected String getFirstRuleName() {
        	return "Model";
       	}

       	@Override
       	protected ReturnMatrixEcoreUnaliasedImplicitGrammarAccess getGrammarAccess() {
       		return grammarAccess;
       	}




    // $ANTLR start "entryRuleModel"
    // InternalReturnMatrixEcoreUnaliasedImplicit.g:64:1: entryRuleModel returns [EObject current=null] : iv_ruleModel= ruleModel EOF ;
    public final EObject entryRuleModel() throws RecognitionException {
        EObject current = null;

        EObject iv_ruleModel = null;


        try {
            // InternalReturnMatrixEcoreUnaliasedImplicit.g:64:46: (iv_ruleModel= ruleModel EOF )
            // InternalReturnMatrixEcoreUnaliasedImplicit.g:65:2: iv_ruleModel= ruleModel EOF
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
    // InternalReturnMatrixEcoreUnaliasedImplicit.g:71:1: ruleModel returns [EObject current=null] : ( (lv_name_0_0= RULE_VALUE ) ) ;
    public final EObject ruleModel() throws RecognitionException {
        EObject current = null;

        Token lv_name_0_0=null;


        	enterRule();

        try {
            // InternalReturnMatrixEcoreUnaliasedImplicit.g:77:2: ( ( (lv_name_0_0= RULE_VALUE ) ) )
            // InternalReturnMatrixEcoreUnaliasedImplicit.g:78:2: ( (lv_name_0_0= RULE_VALUE ) )
            {
            // InternalReturnMatrixEcoreUnaliasedImplicit.g:78:2: ( (lv_name_0_0= RULE_VALUE ) )
            // InternalReturnMatrixEcoreUnaliasedImplicit.g:79:3: (lv_name_0_0= RULE_VALUE )
            {
            // InternalReturnMatrixEcoreUnaliasedImplicit.g:79:3: (lv_name_0_0= RULE_VALUE )
            // InternalReturnMatrixEcoreUnaliasedImplicit.g:80:4: lv_name_0_0= RULE_VALUE
            {
            lv_name_0_0=(Token)match(input,RULE_VALUE,FOLLOW_2); 

            				newLeafNode(lv_name_0_0, grammarAccess.getModelAccess().getNameVALUETerminalRuleCall_0());
            			

            				if (current==null) {
            					current = createModelElement(grammarAccess.getModelRule());
            				}
            				setWithLastConsumed(
            					current,
            					"name",
            					lv_name_0_0,
            					"audit.ReturnMatrixEcoreUnaliasedImplicit.VALUE");
            			

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

    // Delegated rules


 

    public static final BitSet FOLLOW_1 = new BitSet(new long[]{0x0000000000000000L});
    public static final BitSet FOLLOW_2 = new BitSet(new long[]{0x0000000000000002L});

}