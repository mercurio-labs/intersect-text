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
import audit.scopes.services.SignificantCommentDatatypeGrammarAccess;



import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

@SuppressWarnings("all")
public class InternalSignificantCommentDatatypeParser extends AbstractInternalAntlrParser {
    public static final String[] tokenNames = new String[] {
        "<invalid>", "<EOR>", "<DOWN>", "<UP>", "RULE_ID", "RULE_ML_COMMENT", "RULE_WS", "RULE_SL_COMMENT"
    };
    public static final int RULE_ID=4;
    public static final int RULE_WS=6;
    public static final int RULE_SL_COMMENT=7;
    public static final int RULE_ML_COMMENT=5;
    public static final int EOF=-1;

    // delegates
    // delegators


        public InternalSignificantCommentDatatypeParser(TokenStream input) {
            this(input, new RecognizerSharedState());
        }
        public InternalSignificantCommentDatatypeParser(TokenStream input, RecognizerSharedState state) {
            super(input, state);
             
        }
        

    public String[] getTokenNames() { return InternalSignificantCommentDatatypeParser.tokenNames; }
    public String getGrammarFileName() { return "InternalSignificantCommentDatatype.g"; }



     	private SignificantCommentDatatypeGrammarAccess grammarAccess;

        public InternalSignificantCommentDatatypeParser(TokenStream input, SignificantCommentDatatypeGrammarAccess grammarAccess) {
            this(input);
            this.grammarAccess = grammarAccess;
            registerRules(grammarAccess.getGrammar());
        }

        @Override
        protected String getFirstRuleName() {
        	return "Model";
       	}

       	@Override
       	protected SignificantCommentDatatypeGrammarAccess getGrammarAccess() {
       		return grammarAccess;
       	}




    // $ANTLR start "entryRuleModel"
    // InternalSignificantCommentDatatype.g:64:1: entryRuleModel returns [EObject current=null] : iv_ruleModel= ruleModel EOF ;
    public final EObject entryRuleModel() throws RecognitionException {
        EObject current = null;

        EObject iv_ruleModel = null;


        try {
            // InternalSignificantCommentDatatype.g:64:46: (iv_ruleModel= ruleModel EOF )
            // InternalSignificantCommentDatatype.g:65:2: iv_ruleModel= ruleModel EOF
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
    // InternalSignificantCommentDatatype.g:71:1: ruleModel returns [EObject current=null] : ( (lv_name_0_0= ruleWord ) ) ;
    public final EObject ruleModel() throws RecognitionException {
        EObject current = null;

        AntlrDatatypeRuleToken lv_name_0_0 = null;



        	enterRule();

        try {
            // InternalSignificantCommentDatatype.g:77:2: ( ( (lv_name_0_0= ruleWord ) ) )
            // InternalSignificantCommentDatatype.g:78:2: ( (lv_name_0_0= ruleWord ) )
            {
            // InternalSignificantCommentDatatype.g:78:2: ( (lv_name_0_0= ruleWord ) )
            // InternalSignificantCommentDatatype.g:79:3: (lv_name_0_0= ruleWord )
            {
            // InternalSignificantCommentDatatype.g:79:3: (lv_name_0_0= ruleWord )
            // InternalSignificantCommentDatatype.g:80:4: lv_name_0_0= ruleWord
            {

            				newCompositeNode(grammarAccess.getModelAccess().getNameWordParserRuleCall_0());
            			
            pushFollow(FOLLOW_2);
            lv_name_0_0=ruleWord();

            state._fsp--;


            				if (current==null) {
            					current = createModelElementForParent(grammarAccess.getModelRule());
            				}
            				set(
            					current,
            					"name",
            					lv_name_0_0,
            					"audit.scopes.SignificantCommentDatatype.Word");
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


    // $ANTLR start "entryRuleWord"
    // InternalSignificantCommentDatatype.g:100:1: entryRuleWord returns [String current=null] : iv_ruleWord= ruleWord EOF ;
    public final String entryRuleWord() throws RecognitionException {
        String current = null;

        AntlrDatatypeRuleToken iv_ruleWord = null;


        try {
            // InternalSignificantCommentDatatype.g:100:44: (iv_ruleWord= ruleWord EOF )
            // InternalSignificantCommentDatatype.g:101:2: iv_ruleWord= ruleWord EOF
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
        }
        return current;
    }
    // $ANTLR end "entryRuleWord"


    // $ANTLR start "ruleWord"
    // InternalSignificantCommentDatatype.g:107:1: ruleWord returns [AntlrDatatypeRuleToken current=new AntlrDatatypeRuleToken()] : (this_ID_0= RULE_ID this_ML_COMMENT_1= RULE_ML_COMMENT this_ID_2= RULE_ID ) ;
    public final AntlrDatatypeRuleToken ruleWord() throws RecognitionException {
        AntlrDatatypeRuleToken current = new AntlrDatatypeRuleToken();

        Token this_ID_0=null;
        Token this_ML_COMMENT_1=null;
        Token this_ID_2=null;


        	enterRule();

        try {
            // InternalSignificantCommentDatatype.g:113:2: ( (this_ID_0= RULE_ID this_ML_COMMENT_1= RULE_ML_COMMENT this_ID_2= RULE_ID ) )
            // InternalSignificantCommentDatatype.g:114:2: (this_ID_0= RULE_ID this_ML_COMMENT_1= RULE_ML_COMMENT this_ID_2= RULE_ID )
            {
            // InternalSignificantCommentDatatype.g:114:2: (this_ID_0= RULE_ID this_ML_COMMENT_1= RULE_ML_COMMENT this_ID_2= RULE_ID )
            // InternalSignificantCommentDatatype.g:115:3: this_ID_0= RULE_ID this_ML_COMMENT_1= RULE_ML_COMMENT this_ID_2= RULE_ID
            {
            this_ID_0=(Token)match(input,RULE_ID,FOLLOW_3); 

            			current.merge(this_ID_0);
            		

            			newLeafNode(this_ID_0, grammarAccess.getWordAccess().getIDTerminalRuleCall_0());
            		
            this_ML_COMMENT_1=(Token)match(input,RULE_ML_COMMENT,FOLLOW_4); 

            			current.merge(this_ML_COMMENT_1);
            		

            			newLeafNode(this_ML_COMMENT_1, grammarAccess.getWordAccess().getML_COMMENTTerminalRuleCall_1());
            		
            this_ID_2=(Token)match(input,RULE_ID,FOLLOW_2); 

            			current.merge(this_ID_2);
            		

            			newLeafNode(this_ID_2, grammarAccess.getWordAccess().getIDTerminalRuleCall_2());
            		

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
    // $ANTLR end "ruleWord"

    // Delegated rules


 

    public static final BitSet FOLLOW_1 = new BitSet(new long[]{0x0000000000000000L});
    public static final BitSet FOLLOW_2 = new BitSet(new long[]{0x0000000000000002L});
    public static final BitSet FOLLOW_3 = new BitSet(new long[]{0x0000000000000020L});
    public static final BitSet FOLLOW_4 = new BitSet(new long[]{0x0000000000000010L});

}