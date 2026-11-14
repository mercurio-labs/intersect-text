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
import audit.scopes.services.NestedRestoreGrammarAccess;



import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

@SuppressWarnings("all")
public class InternalNestedRestoreParser extends AbstractInternalAntlrParser {
    public static final String[] tokenNames = new String[] {
        "<invalid>", "<EOR>", "<DOWN>", "<UP>", "RULE_ID", "RULE_WS", "RULE_ML_COMMENT", "RULE_SL_COMMENT", "'['", "']'", "'done'", "'a'", "'c'", "'b'"
    };
    public static final int T__9=9;
    public static final int T__8=8;
    public static final int RULE_ID=4;
    public static final int RULE_WS=5;
    public static final int RULE_SL_COMMENT=7;
    public static final int T__11=11;
    public static final int RULE_ML_COMMENT=6;
    public static final int T__12=12;
    public static final int T__13=13;
    public static final int EOF=-1;
    public static final int T__10=10;

    // delegates
    // delegators


        public InternalNestedRestoreParser(TokenStream input) {
            this(input, new RecognizerSharedState());
        }
        public InternalNestedRestoreParser(TokenStream input, RecognizerSharedState state) {
            super(input, state);
             
        }
        

    public String[] getTokenNames() { return InternalNestedRestoreParser.tokenNames; }
    public String getGrammarFileName() { return "InternalNestedRestore.g"; }



     	private NestedRestoreGrammarAccess grammarAccess;

        public InternalNestedRestoreParser(TokenStream input, NestedRestoreGrammarAccess grammarAccess) {
            this(input);
            this.grammarAccess = grammarAccess;
            registerRules(grammarAccess.getGrammar());
        }

        @Override
        protected String getFirstRuleName() {
        	return "Model";
       	}

       	@Override
       	protected NestedRestoreGrammarAccess getGrammarAccess() {
       		return grammarAccess;
       	}




    // $ANTLR start "entryRuleModel"
    // InternalNestedRestore.g:64:1: entryRuleModel returns [EObject current=null] : iv_ruleModel= ruleModel EOF ;
    public final EObject entryRuleModel() throws RecognitionException {
        EObject current = null;

        EObject iv_ruleModel = null;


        try {
            // InternalNestedRestore.g:64:46: (iv_ruleModel= ruleModel EOF )
            // InternalNestedRestore.g:65:2: iv_ruleModel= ruleModel EOF
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
    // InternalNestedRestore.g:71:1: ruleModel returns [EObject current=null] : (otherlv_0= '[' ( (lv_name_1_0= ruleOuter ) ) otherlv_2= ']' otherlv_3= 'done' ) ;
    public final EObject ruleModel() throws RecognitionException {
        EObject current = null;

        Token otherlv_0=null;
        Token otherlv_2=null;
        Token otherlv_3=null;
        AntlrDatatypeRuleToken lv_name_1_0 = null;



        	enterRule();

        try {
            // InternalNestedRestore.g:77:2: ( (otherlv_0= '[' ( (lv_name_1_0= ruleOuter ) ) otherlv_2= ']' otherlv_3= 'done' ) )
            // InternalNestedRestore.g:78:2: (otherlv_0= '[' ( (lv_name_1_0= ruleOuter ) ) otherlv_2= ']' otherlv_3= 'done' )
            {
            // InternalNestedRestore.g:78:2: (otherlv_0= '[' ( (lv_name_1_0= ruleOuter ) ) otherlv_2= ']' otherlv_3= 'done' )
            // InternalNestedRestore.g:79:3: otherlv_0= '[' ( (lv_name_1_0= ruleOuter ) ) otherlv_2= ']' otherlv_3= 'done'
            {
            otherlv_0=(Token)match(input,8,FOLLOW_3); 

            			newLeafNode(otherlv_0, grammarAccess.getModelAccess().getLeftSquareBracketKeyword_0());
            		
            // InternalNestedRestore.g:83:3: ( (lv_name_1_0= ruleOuter ) )
            // InternalNestedRestore.g:84:4: (lv_name_1_0= ruleOuter )
            {
            // InternalNestedRestore.g:84:4: (lv_name_1_0= ruleOuter )
            // InternalNestedRestore.g:85:5: lv_name_1_0= ruleOuter
            {

            					newCompositeNode(grammarAccess.getModelAccess().getNameOuterParserRuleCall_1_0());
            				
            pushFollow(FOLLOW_4);
            lv_name_1_0=ruleOuter();

            state._fsp--;


            					if (current==null) {
            						current = createModelElementForParent(grammarAccess.getModelRule());
            					}
            					set(
            						current,
            						"name",
            						lv_name_1_0,
            						"audit.scopes.NestedRestore.Outer");
            					afterParserOrEnumRuleCall();
            				

            }


            }

            otherlv_2=(Token)match(input,9,FOLLOW_5); 

            			newLeafNode(otherlv_2, grammarAccess.getModelAccess().getRightSquareBracketKeyword_2());
            		
            otherlv_3=(Token)match(input,10,FOLLOW_2); 

            			newLeafNode(otherlv_3, grammarAccess.getModelAccess().getDoneKeyword_3());
            		

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


    // $ANTLR start "entryRuleOuter"
    // InternalNestedRestore.g:114:1: entryRuleOuter returns [String current=null] : iv_ruleOuter= ruleOuter EOF ;
    public final String entryRuleOuter() throws RecognitionException {
        String current = null;

        AntlrDatatypeRuleToken iv_ruleOuter = null;



        	HiddenTokens myHiddenTokenState = ((XtextTokenStream)input).setHiddenTokens();

        try {
            // InternalNestedRestore.g:116:2: (iv_ruleOuter= ruleOuter EOF )
            // InternalNestedRestore.g:117:2: iv_ruleOuter= ruleOuter EOF
            {
             newCompositeNode(grammarAccess.getOuterRule()); 
            pushFollow(FOLLOW_1);
            iv_ruleOuter=ruleOuter();

            state._fsp--;

             current =iv_ruleOuter.getText(); 
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
    // $ANTLR end "entryRuleOuter"


    // $ANTLR start "ruleOuter"
    // InternalNestedRestore.g:126:1: ruleOuter returns [AntlrDatatypeRuleToken current=new AntlrDatatypeRuleToken()] : (kw= 'a' this_Inner_1= ruleInner kw= 'c' ) ;
    public final AntlrDatatypeRuleToken ruleOuter() throws RecognitionException {
        AntlrDatatypeRuleToken current = new AntlrDatatypeRuleToken();

        Token kw=null;
        AntlrDatatypeRuleToken this_Inner_1 = null;



        	enterRule();
        	HiddenTokens myHiddenTokenState = ((XtextTokenStream)input).setHiddenTokens();

        try {
            // InternalNestedRestore.g:133:2: ( (kw= 'a' this_Inner_1= ruleInner kw= 'c' ) )
            // InternalNestedRestore.g:134:2: (kw= 'a' this_Inner_1= ruleInner kw= 'c' )
            {
            // InternalNestedRestore.g:134:2: (kw= 'a' this_Inner_1= ruleInner kw= 'c' )
            // InternalNestedRestore.g:135:3: kw= 'a' this_Inner_1= ruleInner kw= 'c'
            {
            kw=(Token)match(input,11,FOLLOW_6); 

            			current.merge(kw);
            			newLeafNode(kw, grammarAccess.getOuterAccess().getAKeyword_0());
            		

            			newCompositeNode(grammarAccess.getOuterAccess().getInnerParserRuleCall_1());
            		
            pushFollow(FOLLOW_7);
            this_Inner_1=ruleInner();

            state._fsp--;


            			current.merge(this_Inner_1);
            		

            			afterParserOrEnumRuleCall();
            		
            kw=(Token)match(input,12,FOLLOW_2); 

            			current.merge(kw);
            			newLeafNode(kw, grammarAccess.getOuterAccess().getCKeyword_2());
            		

            }


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
    // $ANTLR end "ruleOuter"


    // $ANTLR start "entryRuleInner"
    // InternalNestedRestore.g:162:1: entryRuleInner returns [String current=null] : iv_ruleInner= ruleInner EOF ;
    public final String entryRuleInner() throws RecognitionException {
        String current = null;

        AntlrDatatypeRuleToken iv_ruleInner = null;



        	HiddenTokens myHiddenTokenState = ((XtextTokenStream)input).setHiddenTokens("RULE_WS");

        try {
            // InternalNestedRestore.g:164:2: (iv_ruleInner= ruleInner EOF )
            // InternalNestedRestore.g:165:2: iv_ruleInner= ruleInner EOF
            {
             newCompositeNode(grammarAccess.getInnerRule()); 
            pushFollow(FOLLOW_1);
            iv_ruleInner=ruleInner();

            state._fsp--;

             current =iv_ruleInner.getText(); 
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
    // $ANTLR end "entryRuleInner"


    // $ANTLR start "ruleInner"
    // InternalNestedRestore.g:174:1: ruleInner returns [AntlrDatatypeRuleToken current=new AntlrDatatypeRuleToken()] : kw= 'b' ;
    public final AntlrDatatypeRuleToken ruleInner() throws RecognitionException {
        AntlrDatatypeRuleToken current = new AntlrDatatypeRuleToken();

        Token kw=null;


        	enterRule();
        	HiddenTokens myHiddenTokenState = ((XtextTokenStream)input).setHiddenTokens("RULE_WS");

        try {
            // InternalNestedRestore.g:181:2: (kw= 'b' )
            // InternalNestedRestore.g:182:2: kw= 'b'
            {
            kw=(Token)match(input,13,FOLLOW_2); 

            		current.merge(kw);
            		newLeafNode(kw, grammarAccess.getInnerAccess().getBKeyword());
            	

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
    // $ANTLR end "ruleInner"

    // Delegated rules


 

    public static final BitSet FOLLOW_1 = new BitSet(new long[]{0x0000000000000000L});
    public static final BitSet FOLLOW_2 = new BitSet(new long[]{0x0000000000000002L});
    public static final BitSet FOLLOW_3 = new BitSet(new long[]{0x0000000000000800L});
    public static final BitSet FOLLOW_4 = new BitSet(new long[]{0x0000000000000200L});
    public static final BitSet FOLLOW_5 = new BitSet(new long[]{0x0000000000000400L});
    public static final BitSet FOLLOW_6 = new BitSet(new long[]{0x0000000000002000L});
    public static final BitSet FOLLOW_7 = new BitSet(new long[]{0x0000000000001000L});

}