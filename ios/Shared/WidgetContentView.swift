import SwiftUI

struct WidgetContentView: View {
    let cards: [WidgetCard]
    let medium: Bool
    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("HOLOCRON  ✦").font(.caption2.weight(.semibold)).tracking(2).foregroundStyle(Color(red: 0.61, green: 0.92, blue: 0.86))
            if cards.isEmpty {
                Text("Your galaxy, closer.").font(.title3.weight(.semibold))
                Text("Open Holocron to save a discovery.").font(.caption)
            } else if medium {
                HStack(alignment: .top, spacing: 20) { ForEach(Array(cards.prefix(2).enumerated()), id: \.offset) { _, card in cardView(card) } }
            } else if let card = cards.first { cardView(card) }
        }.frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            .foregroundStyle(Color(red: 0.95, green: 0.95, blue: 0.92))
            .background {
                ZStack {
                    Color(red: 0.06, green: 0.09, blue: 0.1)
                    if let card = cards.last { Image(card.artwork).resizable().scaledToFill().opacity(0.25) }
                    LinearGradient(colors: [.clear, Color(red: 0.06, green: 0.09, blue: 0.1)], startPoint: .top, endPoint: .bottom)
                }
            }

    }
    @ViewBuilder private func cardView(_ card: WidgetCard) -> some View {
        if let url = URL(string: card.url) { Link(destination: url) {
            VStack(alignment: .leading, spacing: 8) {
                Text(card.name).font(.headline).lineLimit(3)
                Text(card.subtitle).font(.caption2).lineLimit(2)
                Text("Explore →").font(.caption).foregroundStyle(Color(red: 0.91, green: 0.73, blue: 0.53))
            }.frame(maxWidth: .infinity, alignment: .leading)
        } }
    }
}
