const pptxgen = require('pptxgenjs');
const path = require('path');
const h2p = require('C:/Users/Lenovo/.claude/plugins/cache/anthropic-agent-skills/example-skills/f23222824449/skills/pptx/scripts/html2pptx.js');

async function createPresentation() {
  const pptx = new pptxgen();
  pptx.layout = 'LAYOUT_16x9';
  pptx.author = 'Aztibase Network';
  pptx.title = 'Aztibase Network — Investor Pitch Deck';

  const slidesDir = path.join(__dirname, 'slides');
  const slideFiles = [
    'slide01-title.html',
    'slide02-problem.html',
    'slide03-solution.html',
    'slide04-howitworks.html',
    'slide05-traction.html',
    'slide06-tokenomics.html',
    'slide07-market.html',
    'slide08-revenue.html',
    'slide09-gtm.html',
    'slide10-competitive.html',
    'slide11-ask.html',
    'slide12-closing.html',
  ];

  for (const file of slideFiles) {
    console.log(`Processing ${file}...`);
    const htmlPath = path.join(slidesDir, file);
    const { slide, placeholders } = await h2p(htmlPath, pptx);

    // Add tokenomics pie chart on slide 6
    if (file === 'slide06-tokenomics.html' && placeholders.length > 0) {
      slide.addChart(pptx.charts.PIE, [{
        name: 'Emission Distribution',
        labels: ['Validators (70%)', 'PoUW Compute (15%)', 'Treasury (10%)', 'Insurance (5%)'],
        values: [70, 15, 10, 5]
      }], {
        ...placeholders[0],
        showPercent: true,
        showLegend: true,
        legendPos: 'b',
        legendFontSize: 7,
        chartColors: ['4A3B8F', '6B5CA5', 'D4A574', 'C4B8E0'],
        dataLabelColor: 'FFFFFF',
        dataLabelFontSize: 8,
      });
    }

    // Add competitive table on slide 10
    if (file === 'slide10-competitive.html' && placeholders.length > 0) {
      const p = placeholders[0];
      const headerStyle = { fill: { color: '4A3B8F' }, color: 'FFFFFF', bold: true, fontSize: 8, align: 'center', valign: 'middle' };
      const yesStyle = { fill: { color: 'EDE8F5' }, color: '3A2D7A', bold: true, fontSize: 8, align: 'center', valign: 'middle' };
      const noStyle = { fill: { color: 'F9F7FA' }, color: '8B7FB0', fontSize: 8, align: 'center', valign: 'middle' };
      const featureStyle = { fill: { color: 'F5F2FA' }, color: '3A2D7A', bold: true, fontSize: 8, align: 'left', valign: 'middle' };

      const tableData = [
        [
          { text: 'Feature', options: headerStyle },
          { text: 'Aztibase', options: headerStyle },
          { text: 'Bittensor', options: headerStyle },
          { text: 'Akash', options: headerStyle },
          { text: 'Render', options: headerStyle },
          { text: 'Ethereum', options: headerStyle },
        ],
        [
          { text: 'AI-Native Consensus', options: featureStyle },
          { text: 'Yes (PoUW)', options: yesStyle },
          { text: 'No', options: noStyle },
          { text: 'No', options: noStyle },
          { text: 'No', options: noStyle },
          { text: 'No (PoS)', options: noStyle },
        ],
        [
          { text: 'On-chain Inference', options: featureStyle },
          { text: 'Yes', options: yesStyle },
          { text: 'No', options: noStyle },
          { text: 'No', options: noStyle },
          { text: 'No', options: noStyle },
          { text: 'No', options: noStyle },
        ],
        [
          { text: 'TPS', options: featureStyle },
          { text: '10,000+', options: yesStyle },
          { text: '~20', options: noStyle },
          { text: 'N/A', options: noStyle },
          { text: 'N/A', options: noStyle },
          { text: '~30', options: noStyle },
        ],
        [
          { text: 'Finality', options: featureStyle },
          { text: '<1 second', options: yesStyle },
          { text: '~12s', options: noStyle },
          { text: 'N/A', options: noStyle },
          { text: 'N/A', options: noStyle },
          { text: '~12 min', options: noStyle },
        ],
        [
          { text: 'Dual VM', options: featureStyle },
          { text: 'WASM+EVM', options: yesStyle },
          { text: 'No', options: noStyle },
          { text: 'No', options: noStyle },
          { text: 'No', options: noStyle },
          { text: 'EVM only', options: noStyle },
        ],
        [
          { text: 'Browser Nodes', options: featureStyle },
          { text: 'Yes (WebRTC)', options: yesStyle },
          { text: 'No', options: noStyle },
          { text: 'No', options: noStyle },
          { text: 'No', options: noStyle },
          { text: 'No', options: noStyle },
        ],
        [
          { text: 'Language', options: featureStyle },
          { text: 'Pure Rust', options: yesStyle },
          { text: 'Python', options: noStyle },
          { text: 'Go', options: noStyle },
          { text: 'N/A', options: noStyle },
          { text: 'Multiple', options: noStyle },
        ],
      ];

      slide.addTable(tableData, {
        x: p.x, y: p.y, w: p.w, h: p.h,
        colW: [1.4, 1.2, 1.0, 1.0, 1.0, 1.0],
        border: { pt: 0.5, color: 'E0DCE8' },
        rowH: [0.4, 0.35, 0.35, 0.35, 0.35, 0.35, 0.35, 0.35],
      });
    }
  }

  const outPath = path.join(__dirname, '..', 'docs', 'AZTIBASE_INVESTOR_DECK.pptx');
  await pptx.writeFile({ fileName: outPath });
  console.log(`Presentation saved to ${outPath}`);
}

createPresentation().catch(err => {
  console.error('Error:', err.message);
  process.exit(1);
});
