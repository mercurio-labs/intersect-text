package audit.scopes.parser.antlr.internal;

import org.eclipse.xtext.*;
import org.eclipse.xtext.parser.*;
import org.eclipse.xtext.parser.impl.*;
import org.eclipse.emf.ecore.util.EcoreUtil;
import org.eclipse.emf.ecore.EObject;
import org.eclipse.xtext.parser.antlr.AbstractInternalAntlrParser;
import org.eclipse.xtext.parser.antlr.XtextTokenStream;
import org.eclipse.xtext.parser.antlr.XtextTokenStream.HiddenTokens;
import org.eclipse.xtext.parser.antlr.AntlrDatatypeRuleToken;
import audit.scopes.services.OverrideTokenChildGrammarAccess;



import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

@SuppressWarnings("all")
public class InternalOverrideTokenChildParser extends AbstractInternalAntlrParser {
    public static final String[] tokenNames = new String[] {
        "<invalid>", "<EOR>", "<DOWN>", "<UP>", "RULE_ID", "RULE_WS", "RULE_ML_COMMENT", "RULE_SL_COMMENT"
    };
    public static final int RULE_ID=4;
    public static final int RULE_WS=5;
    public static final int RULE_SL_COMMENT=7;
    public static final int RULE_ML_COMMENT=6;
    public static final int EOF=-1;

    // delegates
    // delegators


        public InternalOverrideTokenChildParser(TokenStream input) {
            this(input, new RecognizerSharedState());
        }
        public InternalOverrideTokenChildParser(TokenStream input, RecognizerSharedState state) {
            super(input, state);
             
        }
        

    public String[] getTokenNames() { return InternalOverrideTokenChildParser.tokenNames; }
    public String getGrammarFileName() { return "InternalOverrideTokenChild.g"; }



     	private OverrideTokenChildGrammarAccess grammarAccess;

        public InternalOverrideTokenChildParser(TokenStream input, OverrideTokenChildGrammarAccess grammarAccess) {
            this(input);
            this.grammarAccess = grammarAccess;
            registerRules(grammarAccess.getGrammar());
        }

        @Override
        protected String getFirstRuleName() {
        	return "Entry";
       	}

       	@Override
       	protected OverrideTokenChildGrammarAccess getGrammarAccess() {
       		return grammarAccess;
       	}




    // $ANTLR start "entryRuleEntry"
    // InternalOverrideTokenChild.g:64:1: entryRuleEntry returns [EObject current=null] : iv_ruleEntry= ruleEntry EOF ;
    public final EObject entryRuleEntry() throws RecognitionException {
        EObject current = null;

        EObject iv_ruleEntry = null;


        try {
            // InternalOverrideTokenChild.g:64:46: (iv_ruleEntry= ruleEntry EOF )
            // InternalOverrideTokenChild.g:65:2: iv_ruleEntry= ruleEntry EOF
            {
             newCompositeNode(grammarAccess.getEntryRule()); 
            pushFollow(FOLLOW_1);
            iv_ruleEntry=ruleEntry();

            state._fsp--;

             current =iv_ruleEntry; 
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
    // $ANTLR end "entryRuleEntry"


    // $ANTLR start "ruleEntry"
    // InternalOverrideTokenChild.g:71:1: ruleEntry returns [EObject current=null] : this_Model_0= ruleModel ;
    public final EObject ruleEntry() throws RecognitionException {
        EObject current = null;

        EObject this_Model_0 = null;



        	enterRule();

        try {
            // InternalOverrideTokenChild.g:77:2: (this_Model_0= ruleModel )
            // InternalOverrideTokenChild.g:78:2: this_Model_0= ruleModel
            {

            		newCompositeNode(grammarAccess.getEntryAccess().getModelParserRuleCall());
            	
            pushFollow(FOLLOW_2);
            this_Model_0=ruleModel();

            state._fsp--;


            		current = this_Model_0;
            		afterParserOrEnumRuleCall();
            	

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
    // $ANTLR end "ruleEntry"


    // $ANTLR start "entryRuleModel"
    // InternalOverrideTokenChild.g:89:1: entryRuleModel returns [EObject current=null] : iv_ruleModel= ruleModel EOF ;
    public final EObject entryRuleModel() throws RecognitionException {
        EObject current = null;

        EObject iv_ruleModel = null;


        try {
            // InternalOverrideTokenChild.g:89:46: (iv_ruleModel= ruleModel EOF )
            // InternalOverrideTokenChild.g:90:2: iv_ruleModel= ruleModel EOF
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
    // InternalOverrideTokenChild.g:96:1: ruleModel returns [EObject current=null] : ( (lv_name_0_0= RULE_ID ) ) ;
    public final EObject ruleModel() throws RecognitionException {
        EObject current = null;

        Token lv_name_0_0=null;


        	enterRule();

        try {
            // InternalOverrideTokenChild.g:102:2: ( ( (lv_name_0_0= RULE_ID ) ) )
            // InternalOverrideTokenChild.g:103:2: ( (lv_name_0_0= RULE_ID ) )
            {
            // InternalOverrideTokenChild.g:103:2: ( (lv_name_0_0= RULE_ID ) )
            // InternalOverrideTokenChild.g:104:3: (lv_name_0_0= RULE_ID )
            {
            // InternalOverrideTokenChild.g:104:3: (lv_name_0_0= RULE_ID )
            // InternalOverrideTokenChild.g:105:4: lv_name_0_0= RULE_ID
            {
            lv_name_0_0=(Token)match(input,RULE_ID,FOLLOW_2); 

            				newLeafNode(lv_name_0_0, grammarAccess.getModelAccess().getNameIDTerminalRuleCall_0());
            			

            				if (current==null) {
            					current = createModelElement(grammarAccess.getModelRule());
            				}
            				setWithLastConsumed(
            					current,
            					"name",
            					lv_name_0_0,
            					"audit.scopes.OverrideTokenBase.ID");
            			

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