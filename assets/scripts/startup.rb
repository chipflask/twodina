require_relative 'prelude'

# An example of using classes without the DSL.
class Game < BaseGame
  attr_accessor :shared_score

  def initialize()
    super
    @shared_score = 0
  end

  on_new_game do
    puts "new game: self=#{self.inspect}"
    dialogue :Start
  end

end

# class Player < BasePlayer
#   attr_accessor :num_gems

#   def initialize()
#     super
#     @num_gems = 0
#   end
# end

player do
  define :num_gems

  setup do
    @num_gems = 0
  end

  on_collect :gem do
    @num_gems += 1
  end
  # on_collect(:big_gem) { @num_gems += 5 }
  on_collect :any do
    total = players.map {|pl| pl.num_gems }.sum
    puts "total gems: #{total}"
    map.show :secret_door if total > 8
  end
end

map do
  define :gems_found, :total_gems

  setup do
    @gems_found = 0
    @total_gems = 0
  end

  on_load do
    puts "loaded map: self=#{self.inspect}"
  end

  on_enter do
    puts "entered map: self=#{self.inspect}"
  end

  on_exit do
    puts "exited map: self=#{self.inspect}"
  end
end

# This is optional, in case you want to use custom parameters for the
# constructor.
# game = Game.new
