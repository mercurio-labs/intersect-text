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
import audit.scopes.services.RestoreChildPolicyGrammarAccess;



import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

@SuppressWarnings("all")
public class InternalRestoreChildPolicyParser extends AbstractInternalAntlrParser {
    public static final String[] tokenNames = new String[] {
        "<invalid>", "<EOR>", "<DOWN>", "<UP>", "RULE_WS", "RULE_ID", "RULE_ML_COMMENT", "RULE_SL_COMMENT", "'['", "']'", "'done'"
    };
    public static final int T__9=9;
    public static final int T__8=8;
    public static final int RULE_ID=5;
    public static final int RULE_WS=4;
    public static final int RULE_SL_COMMENT=7;
    public static final int RULE_ML_COMMENT=6;
    public static final int EOF=-1;
    public static final int T__10=10;

    // delegates
    // delegators


        public InternalRestoreChildPolicyParser(TokenStream input) {
            this(input, new RecognizerSharedState());
        }
        public InternalRestoreChildPolicyParser(TokenStream input, RecognizerSharedState state) {
            super(input, state);
             
        }
        

    public String[] getTokenNames() { return InternalRestoreChildPolicyParser.tokenNames; }
    public String getGrammarFileName() { return "InternalRestoreChildPolicy.g"; }



     	private RestoreChildPolicyGrammarAccess grammarAccess;

        public InternalRestoreChildPolicyParser(TokenStream input, RestoreChildPolicyGrammarAccess grammarAccess) {
            this(input);
            this.grammarAccess = grammarAccess;
            registerRules(grammarAccess.getGrammar());
        }

        @Override
        protected String getFirstRuleName() {
        	return "Model";
       	}

       	@Override
       	protected RestoreChildPolicyGrammarAccess getGrammarAccess() {
       		return grammarAccess;
       	}




    // $ANTLR start "entryRuleModel"
    // InternalRestoreChildPolicy.g:64:1: entryRuleModel returns [EObject current=null] : iv_ruleModel= ruleModel EOF ;
    public final EObject entryRuleModel() throws RecognitionException {
        EObject current = null;

        EObject iv_ruleModel = null;


        try {
            // InternalRestoreChildPolicy.g:64:46: (iv_ruleModel= ruleModel EOF )
            // InternalRestoreChildPolicy.g:65:2: iv_ruleModel= ruleModel EOF
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
    // InternalRestoreChildPolicy.g:71:1: ruleModel returns [EObject current=null] : (otherlv_0= '[' ( (lv_name_1_0= ruleWord ) ) otherlv_2= ']' this_WS_3= RULE_WS otherlv_4= 'done' ) ;
    public final EObject ruleModel() throws RecognitionException {
        EObject current = null;

        Token otherlv_0=null;
        Token otherlv_2=null;
        Token this_WS_3=null;
        Token otherlv_4=null;
        AntlrDatatypeRuleToken lv_name_1_0 = null;



        	enterRule();

        try {
            // InternalRestoreChildPolicy.g:77:2: ( (otherlv_0= '[' ( (lv_name_1_0= ruleWord ) ) otherlv_2= ']' this_WS_3= RULE_WS otherlv_4= 'done' ) )
            // InternalRestoreChildPolicy.g:78:2: (otherlv_0= '[' ( (lv_name_1_0= ruleWord ) ) otherlv_2= ']' this_WS_3= RULE_WS otherlv_4= 'done' )
            {
            // InternalRestoreChildPolicy.g:78:2: (otherlv_0= '[' ( (lv_name_1_0= ruleWord ) ) otherlv_2= ']' this_WS_3= RULE_WS otherlv_4= 'done' )
            // InternalRestoreChildPolicy.g:79:3: otherlv_0= '[' ( (lv_name_1_0= ruleWord ) ) otherlv_2= ']' this_WS_3= RULE_WS otherlv_4= 'done'
            {
            otherlv_0=(Token)match(input,8,FOLLOW_3); 

            			newLeafNode(otherlv_0, grammarAccess.getModelAccess().getLeftSquareBracketKeyword_0());
            		
            // InternalRestoreChildPolicy.g:83:3: ( (lv_name_1_0= ruleWord ) )
            // InternalRestoreChildPolicy.g:84:4: (lv_name_1_0= ruleWord )
            {
            // InternalRestoreChildPolicy.g:84:4: (lv_name_1_0= ruleWord )
            // InternalRestoreChildPolicy.g:85:5: lv_name_1_0= ruleWord
            {

            					newCompositeNode(grammarAccess.getModelAccess().getNameWordParserRuleCall_1_0());
            				
            pushFollow(FOLLOW_4);
            lv_name_1_0=ruleWord();

            state._fsp--;


            					if (current==null) {
            						current = createModelElementForParent(grammarAccess.getModelRule());
            					}
            					set(
            						current,
            						"name",
            						lv_name_1_0,
            						"audit.scopes.RestoreChildPolicy.Word");
            					afterParserOrEnumRuleCall();
            				

            }


            }

            otherlv_2=(Token)match(input,9,FOLLOW_5); 

            			newLeafNode(otherlv_2, grammarAccess.getModelAccess().getRightSquareBracketKeyword_2());
            		
            this_WS_3=(Token)match(input,RULE_WS,FOLLOW_6); 

            			newLeafNode(this_WS_3, grammarAccess.getModelAccess().getWSTerminalRuleCall_3());
            		
            otherlv_4=(Token)match(input,10,FOLLOW_2); 

            			newLeafNode(otherlv_4, grammarAccess.getModelAccess().getDoneKeyword_4());
            		

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


    // $ANTLR start "entryRuleWord"
    // InternalRestoreChildPolicy.g:118:1: entryRuleWord returns [String current=null] : iv_ruleWord= ruleWord EOF ;
    public final String entryRuleWord() throws RecognitionException {
        String current = null;

        AntlrDatatypeRuleToken iv_ruleWord = null;



        	HiddenTokens myHiddenTokenState = ((XtextTokenStream)input).setHiddenTokens("RULE_WS", "RULE_ML_COMMENT");

        try {
            // InternalRestoreChildPolicy.g:120:2: (iv_ruleWord= ruleWord EOF )
            // InternalRestoreChildPolicy.g:121:2: iv_ruleWord= ruleWord EOF
            {
             newCompositeNode(grammarAccess.getWordRule()); 
            pushFollow(FOLLOW_1);
            iv_ruleWord=ruleWord();

            state._fsp--;

             current =iv_ruleWord.getText(); 
            match(input,EOF,FOLLOW_2); 

            }

        }

            catch (RecognitionException re) {
                recover(input,re);
                appendSkippedTokens();
            }
        finally {

            	myHiddenTokenState.restore();

        }
        return current;
    }
    // $ANTLR end "entryRuleWord"


    // $ANTLR start "ruleWord"
    // InternalRestoreChildPolicy.g:130:1: ruleWord returns [AntlrDatatypeRuleToken current=new AntlrDatatypeRuleToken()] : this_ID_0= RULE_ID ;
    public final AntlrDatatypeRuleToken ruleWord() throws RecognitionException {
        AntlrDatatypeRuleToken current = new AntlrDatatypeRuleToken();

        Token this_ID_0=null;


        	enterRule();
        	HiddenTokens myHiddenTokenState = ((XtextTokenStream)input).setHiddenTokens("RULE_WS", "RULE_ML_COMMENT");

        try {
            // InternalRestoreChildPolicy.g:137:2: (this_ID_0= RULE_ID )
            // InternalRestoreChildPolicy.g:138:2: this_ID_0= RULE_ID
            {
            this_ID_0=(Token)match(input,RULE_ID,FOLLOW_2); 

            		current.merge(this_ID_0);
            	

            		newLeafNode(this_ID_0, grammarAccess.getWordAccess().getIDTerminalRuleCall());
            	

            }


            	leaveRule();

        }

            catch (RecognitionException re) {
                recover(input,re);
                appendSkippedTokens();
            }
        finally {

            	myHiddenTokenState.restore();

        }
        return current;
    }
    // $ANTLR end "ruleWord"

    // Delegated rules


 

    public static final BitSet FOLLOW_1 = new BitSet(new long[]{0x0000000000000000L});
    public static final BitSet FOLLOW_2 = new BitSet(new long[]{0x0000000000000002L});
    public static final BitSet FOLLOW_3 = new BitSet(new long[]{0x0000000000000020L});
    public static final BitSet FOLLOW_4 = new BitSet(new long[]{0x0000000000000200L});
    public static final BitSet FOLLOW_5 = new BitSet(new long[]{0x0000000000000010L});
    public static final BitSet FOLLOW_6 = new BitSet(new long[]{0x0000000000000400L});

}